use super::repository::Item;

pub(super) fn caption(item: &Item, language: &str) -> String {
    let price = item.price.map_or_else(
        || {
            if language == "en" {
                "Price unavailable"
            } else {
                "Preço não informado"
            }
            .into()
        },
        |cents| format!("R$ {},{:02}", cents / 100, cents % 100),
    );
    let mut text = format!("{}\n<b>{}</b>", escape(&item.title), price);

    if let Some(url) = offer_link(&item.message.text).or_else(|| {
        item.message
            .message_link
            .clone()
            .filter(|url| valid_link(url))
    }) {
        let label = if language == "en" {
            "View offer"
        } else {
            "Ver oferta"
        };
        text.push_str(&format!("\n<a href=\"{}\">{label}</a>", escape(&url)));
    }
    text
}

pub(super) fn separator(day: &str) -> String {
    format!("<b>━━━━ 📅 {} ━━━━</b>", escape(day))
}

pub(super) fn offer_link(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|part| {
            part.trim_matches(|character| {
                matches!(character, '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';')
            })
        })
        .find(|url| valid_link(url))
        .map(str::to_owned)
}

fn valid_link(url: &str) -> bool {
    tauri::Url::parse(url)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
