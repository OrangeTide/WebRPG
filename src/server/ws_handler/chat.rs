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

/// Parse dice expressions like "2d6+3", "1d20", "4d8-2"
pub fn parse_and_roll(expression: &str) -> Result<(Vec<i32>, i32), String> {
    use rand::Rng;

    let expr = expression.trim().to_lowercase();

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
