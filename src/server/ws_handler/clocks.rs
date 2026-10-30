use crate::models::ProgressClockInfo;

/// Smallest and largest number of segments a clock may have.
const MIN_SEGMENTS: i32 = 2;
const MAX_SEGMENTS: i32 = 24;

/// Load every progress clock for a session, ordered for display.
pub fn load_clocks(session_id: i32) -> Vec<ProgressClockInfo> {
    use crate::db;
    use crate::models::db_models::ProgressClock;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let rows: Vec<ProgressClock> = progress_clocks::table
        .filter(progress_clocks::session_id.eq(session_id))
        .order((progress_clocks::sort_order.asc(), progress_clocks::id.asc()))
        .select(ProgressClock::as_select())
        .load(conn)
        .unwrap_or_default();

    rows.into_iter()
        .map(|c| ProgressClockInfo {
            id: c.id,
            label: c.label,
            description: c.description,
            segments: c.segments,
            filled: c.filled,
            filled_color: c.filled_color,
            unfilled_color: c.unfilled_color,
            locked: c.locked,
            sort_order: c.sort_order,
        })
        .collect()
}

/// Create a clock. Returns the updated clock list for the session.
pub fn create_clock(
    session_id: i32,
    label: &str,
    segments: i32,
    filled_color: &str,
    unfilled_color: &str,
) -> Result<Vec<ProgressClockInfo>, String> {
    use crate::db;
    use crate::models::db_models::NewProgressClock;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let segments = segments.clamp(MIN_SEGMENTS, MAX_SEGMENTS);
    let label = label.trim();
    let label = if label.is_empty() { "Clock" } else { label };

    let conn = &mut db::get_conn();

    // Place the new clock after the existing ones.
    let next_order: i32 = progress_clocks::table
        .filter(progress_clocks::session_id.eq(session_id))
        .select(diesel::dsl::max(progress_clocks::sort_order))
        .first::<Option<i32>>(conn)
        .unwrap_or(None)
        .map(|m| m + 1)
        .unwrap_or(0);

    diesel::insert_into(progress_clocks::table)
        .values(&NewProgressClock {
            session_id,
            label,
            segments,
            filled_color,
            unfilled_color,
            sort_order: next_order,
        })
        .execute(conn)
        .map_err(|e| e.to_string())?;

    Ok(load_clocks(session_id))
}

/// Look up a clock's session and locked flag, if it exists.
fn clock_session_and_lock(clock_id: i32) -> Option<(i32, bool)> {
    use crate::db;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    progress_clocks::table
        .find(clock_id)
        .select((progress_clocks::session_id, progress_clocks::locked))
        .first::<(i32, bool)>(conn)
        .optional()
        .unwrap_or(None)
}

/// Set how many segments of a clock are filled. Rejected while the clock is
/// locked. Returns the updated clock list.
pub fn set_filled(
    session_id: i32,
    clock_id: i32,
    filled: i32,
) -> Result<Vec<ProgressClockInfo>, String> {
    use crate::db;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let Some((owner_session, locked)) = clock_session_and_lock(clock_id) else {
        return Err("Clock not found".into());
    };
    if owner_session != session_id {
        return Err("Clock not found".into());
    }
    if locked {
        return Err("This clock is locked".into());
    }

    let conn = &mut db::get_conn();
    let segments: i32 = progress_clocks::table
        .find(clock_id)
        .select(progress_clocks::segments)
        .first(conn)
        .map_err(|e| e.to_string())?;
    let filled = filled.clamp(0, segments);

    diesel::update(progress_clocks::table.find(clock_id))
        .set(progress_clocks::filled.eq(filled))
        .execute(conn)
        .map_err(|e| e.to_string())?;

    Ok(load_clocks(session_id))
}

/// Edit a clock's label and description. Rejected while the clock is locked.
pub fn edit_clock(
    session_id: i32,
    clock_id: i32,
    label: &str,
    description: &str,
) -> Result<Vec<ProgressClockInfo>, String> {
    use crate::db;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let Some((owner_session, locked)) = clock_session_and_lock(clock_id) else {
        return Err("Clock not found".into());
    };
    if owner_session != session_id {
        return Err("Clock not found".into());
    }
    if locked {
        return Err("This clock is locked".into());
    }

    let label = label.trim();
    let label = if label.is_empty() { "Clock" } else { label };

    let conn = &mut db::get_conn();
    diesel::update(progress_clocks::table.find(clock_id))
        .set((
            progress_clocks::label.eq(label),
            progress_clocks::description.eq(description),
        ))
        .execute(conn)
        .map_err(|e| e.to_string())?;

    Ok(load_clocks(session_id))
}

/// Lock or unlock a clock. Returns the updated clock list.
pub fn set_lock(
    session_id: i32,
    clock_id: i32,
    locked: bool,
) -> Result<Vec<ProgressClockInfo>, String> {
    use crate::db;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let Some((owner_session, _)) = clock_session_and_lock(clock_id) else {
        return Err("Clock not found".into());
    };
    if owner_session != session_id {
        return Err("Clock not found".into());
    }

    let conn = &mut db::get_conn();
    diesel::update(progress_clocks::table.find(clock_id))
        .set(progress_clocks::locked.eq(locked))
        .execute(conn)
        .map_err(|e| e.to_string())?;

    Ok(load_clocks(session_id))
}

/// Delete a clock. Returns the updated clock list.
pub fn delete_clock(session_id: i32, clock_id: i32) -> Result<Vec<ProgressClockInfo>, String> {
    use crate::db;
    use crate::schema::progress_clocks;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    diesel::delete(
        progress_clocks::table
            .filter(progress_clocks::id.eq(clock_id))
            .filter(progress_clocks::session_id.eq(session_id)),
    )
    .execute(conn)
    .map_err(|e| e.to_string())?;

    Ok(load_clocks(session_id))
}
