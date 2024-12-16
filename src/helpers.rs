use chrono::NaiveDateTime;
use rocket_dyn_templates::handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext,
};

pub fn register_helpers(handlebars: &mut Handlebars) {
    // Existing helpers
    handlebars.register_helper("format_date", Box::new(format_date));
    handlebars.register_helper("truncate", Box::new(truncate));

    // New pagination helpers
    handlebars.register_helper("add", Box::new(add));
    handlebars.register_helper("subtract", Box::new(subtract));
    handlebars.register_helper("gt", Box::new(gt));
    handlebars.register_helper("lt", Box::new(lt));
}

fn format_date(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(timestamp) = param.value().as_str() {
            if let Ok(date) = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S%.f") {
                let formatted = date.format("%B %d, %Y at %H:%M").to_string();
                out.write(&formatted)?;
                return Ok(());
            }
        }
    }
    out.write("Invalid date")?;
    Ok(())
}

fn truncate(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let length: usize = h.param(1).and_then(|v| v.value().as_u64()).unwrap_or(100) as usize;

    let truncated = if text.chars().count() > length {
        format!("{}...", text.chars().take(length).collect::<String>())
    } else {
        text.to_string()
    };

    out.write(&truncated)?;
    Ok(())
}

fn add(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_i64()).unwrap_or(0);
    let param2 = h.param(1).and_then(|v| v.value().as_i64()).unwrap_or(0);

    out.write(&(param1 + param2).to_string())?;
    Ok(())
}

fn subtract(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_i64()).unwrap_or(0);
    let param2 = h.param(1).and_then(|v| v.value().as_i64()).unwrap_or(0);

    out.write(&(param1 - param2).to_string())?;
    Ok(())
}

fn gt(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_i64()).unwrap_or(0);
    let param2 = h.param(1).and_then(|v| v.value().as_i64()).unwrap_or(0);

    out.write(&(param1 > param2).to_string())?;
    Ok(())
}

fn lt(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_i64()).unwrap_or(0);
    let param2 = h.param(1).and_then(|v| v.value().as_i64()).unwrap_or(0);

    out.write(&(param1 < param2).to_string())?;
    Ok(())
}
