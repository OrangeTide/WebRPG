use crate::models::ChatMessageInfo;

pub fn save_chat_message(
    session_id: i32,
    user_id: i32,
    username: &str,
    message: &str,
    is_dice_roll: bool,
    dice_result: Option<&str>,
) -> ChatMessageInfo {
    use crate::db;
    use crate::models::db_models::NewChatMessage;
    use crate::schema::chat_messages;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let new_msg = NewChatMessage {
        session_id,
        user_id,
        message,
        is_dice_roll,
        dice_result,
    };

    let _ = diesel::insert_into(chat_messages::table)
        .values(&new_msg)
        .execute(conn);

    let id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .unwrap_or(0);

    ChatMessageInfo {
        id,
        username: username.to_string(),
        message: message.to_string(),
        is_dice_roll,
        dice_result: dice_result.map(|s| s.to_string()),
        created_at: String::new(),
    }
}

/// Resolve a check: roll the system's dice, add the ability and the bonuses
/// the player picked, and work out the margin if a DS was given.
///
/// The margin is the interesting number. In Tunnel Goons and its relatives the
/// gap between the total and the DS is damage, dealt to whoever lost, so the
/// summary states it in those terms rather than just pass or fail.
pub fn roll_check(
    label: &str,
    dice: &str,
    ability: &str,
    ability_mod: i32,
    bonuses: &[crate::ws::messages::RollBonus],
    ds: Option<i32>,
    dangerous: bool,
) -> Result<(String, crate::ws::messages::CheckResult), String> {
    use crate::ws::messages::CheckResult;

    let dice_expr = if dice.trim().is_empty() { "2d6" } else { dice };
    let (rolls, dice_total) = parse_and_roll(dice_expr)?;

    let bonus_total: i32 = bonuses.iter().map(|b| b.value).sum();
    let total = dice_total + ability_mod + bonus_total;
    let margin = ds.map(|ds| total - ds);

    // "2d6 (3+4) + Brute 2 + hammer 1 = 10"
    let rolled: Vec<String> = rolls.iter().map(|r| r.to_string()).collect();
    let mut parts = format!("{dice_expr} ({})", rolled.join("+"));
    if !ability.is_empty() || ability_mod != 0 {
        let name = if ability.is_empty() {
            "ability"
        } else {
            ability
        };
        parts.push_str(&format!(" {name} {ability_mod:+}"));
    }
    for bonus in bonuses {
        parts.push_str(&format!(" {} {:+}", bonus.label, bonus.value));
    }

    let mut summary = if label.trim().is_empty() {
        format!("{parts} = {total}")
    } else {
        format!("{label}: {parts} = {total}")
    };

    if let (Some(ds), Some(margin)) = (ds, margin) {
        if margin >= 0 {
            summary.push_str(&format!(" vs DS {ds} - success by {margin}"));
            if dangerous {
                summary.push_str(&format!(", {margin} damage dealt"));
            }
        } else {
            let short = -margin;
            summary.push_str(&format!(" vs DS {ds} - short by {short}"));
            if dangerous {
                summary.push_str(&format!(", {short} damage taken"));
            }
        }
    }

    let result = CheckResult {
        kind: "check".to_string(),
        label: label.to_string(),
        dice: dice_expr.to_string(),
        rolls,
        ability: ability.to_string(),
        ability_mod,
        bonuses: bonuses.to_vec(),
        total,
        ds,
        margin,
        dangerous,
    };

    Ok((summary, result))
}

/// Roll a digit-concatenation die: d66 is two d6 read as tens and units, d666
/// is three. These index the lookup tables common in OSR material, where entry
/// 315 means the dice came up 3, 1, 5.
fn roll_digit_die(digits: u32) -> (Vec<i32>, i32) {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let rolls: Vec<i32> = (0..digits).map(|_| rng.gen_range(1..=6)).collect();
    let total = rolls.iter().fold(0, |acc, r| acc * 10 + r);
    (rolls, total)
}

/// Parse dice expressions like "2d6+3", "1d20", "4d8-2".
///
/// `d66` and `d666` are read as digit dice rather than as one huge die, since
/// that is what those notations mean on a table.
pub fn parse_and_roll(expression: &str) -> Result<(Vec<i32>, i32), String> {
    use rand::Rng;

    let expr = expression.trim().to_lowercase();

    match expr.as_str() {
        "d66" | "1d66" => return Ok(roll_digit_die(2)),
        "d666" | "1d666" => return Ok(roll_digit_die(3)),
        _ => {}
    }

    let (dice_part, modifier) = if let Some(pos) = expr.rfind('+') {
        let (d, m) = expr.split_at(pos);
        let modifier: i32 = m[1..]
            .parse()
            .map_err(|_| format!("Invalid modifier in '{expression}'"))?;
        (d, modifier)
    } else if let Some(pos) = expr.rfind('-') {
        if pos == 0 {
            return Err(format!("Invalid expression: '{expression}'"));
        }
        let (d, m) = expr.split_at(pos);
        let modifier: i32 = m
            .parse()
            .map_err(|_| format!("Invalid modifier in '{expression}'"))?;
        (d, modifier)
    } else {
        (expr.as_str(), 0)
    };

    let parts: Vec<&str> = dice_part.split('d').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid dice expression: '{expression}'"));
    }

    let count: i32 = if parts[0].is_empty() {
        1
    } else {
        parts[0]
            .parse()
            .map_err(|_| format!("Invalid dice count in '{expression}'"))?
    };

    let sides: i32 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid dice sides in '{expression}'"))?;

    if count < 1 || count > 100 || sides < 1 || sides > 1000 {
        return Err("Dice values out of range".to_string());
    }

    let mut rng = rand::thread_rng();
    let rolls: Vec<i32> = (0..count).map(|_| rng.gen_range(1..=sides)).collect();
    let total: i32 = rolls.iter().sum::<i32>() + modifier;

    Ok((rolls, total))
}

#[cfg(test)]
mod tests {
    use super::parse_and_roll;

    #[test]
    fn digit_dice_read_as_table_entries() {
        for expr in ["d666", "1d666"] {
            let (rolls, total) = parse_and_roll(expr).expect("d666 should parse");
            assert_eq!(rolls.len(), 3);
            assert!(rolls.iter().all(|r| (1..=6).contains(r)));
            // Every digit is 1-6, so the total is a valid table key.
            assert_eq!(
                total,
                rolls[0] * 100 + rolls[1] * 10 + rolls[2],
                "digits should read in order"
            );
            assert!((111..=666).contains(&total));
        }

        let (rolls, total) = parse_and_roll("d66").expect("d66 should parse");
        assert_eq!(rolls.len(), 2);
        assert!((11..=66).contains(&total));
    }

    #[test]
    fn ordinary_dice_still_parse() {
        let (rolls, total) = parse_and_roll("2d6+3").expect("2d6+3 should parse");
        assert_eq!(rolls.len(), 2);
        assert!((5..=15).contains(&total));

        assert!(parse_and_roll("d20").is_ok());
        assert!(parse_and_roll("nonsense").is_err());
    }
}
