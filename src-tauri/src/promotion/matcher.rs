use super::{ProductMatch, SourceMessage};
use crate::watch::Watch;
use std::collections::HashSet;

pub(crate) fn match_messages(messages: &[SourceMessage], watches: &[Watch]) -> Vec<ProductMatch> {
    let mut matches = Vec::new();
    for message in messages {
        let matching: Vec<_> = watches
            .iter()
            .filter(|watch| matches_name(&message.text, watch))
            .collect();
        let prices = prices(&message.text);
        let price_lines: HashSet<_> = prices.iter().map(|price| price.line).collect();

        for watch in &matching {
            let related: Vec<_> = prices
                .iter()
                .filter(|price| {
                    let line_products: Vec<_> = matching
                        .iter()
                        .copied()
                        .filter(|product| matches_name(price.line, product))
                        .collect();
                    (matches_name(price.line, watch)
                        && share_product_name(price.line, &line_products))
                        || (line_products.is_empty()
                            && price_lines.len() <= 1
                            && share_product_name(&message.text, &matching))
                })
                .collect();
            let minimum = watch.minimum_price_cents();
            let price = related
                .iter()
                .map(|price| price.cents)
                .filter(|price| {
                    *price >= minimum
                        && watch
                            .max_price_cents
                            .is_none_or(|ceiling| *price <= ceiling)
                })
                .min();
            if (watch.max_price_cents.is_some() || minimum > 0) && price.is_none() {
                continue;
            }

            matches.push(ProductMatch {
                watch_id: watch.id,
                price_cents: price,
                message: message.clone(),
            });
        }
    }
    matches
}

fn share_product_name(text: &str, watches: &[&Watch]) -> bool {
    let words = name_tokens(text);
    let names: Vec<Vec<_>> = watches
        .iter()
        .map(|watch| {
            watch
                .phrases
                .iter()
                .map(|phrase| name_tokens(phrase))
                .filter(|tokens| !tokens.is_empty() && tokens.is_subset(&words))
                .collect()
        })
        .collect();

    // A specific matching name can share its price with broader names of the same product.
    names.iter().flatten().any(|specific| {
        names
            .iter()
            .all(|alternatives| alternatives.iter().any(|name| name.is_subset(specific)))
    })
}

fn matches_name(text: &str, watch: &Watch) -> bool {
    let words = name_tokens(text);

    watch.phrases.iter().any(|phrase| {
        let tokens = name_tokens(phrase);
        !tokens.is_empty() && tokens.is_subset(&words)
    })
}

fn name_tokens(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

struct Price<'a> {
    cents: i64,
    line: &'a str,
}

fn prices(text: &str) -> Vec<Price<'_>> {
    let mut prices = Vec::new();
    for line in text.lines() {
        let normalized = line.to_lowercase();
        for (position, _) in normalized.match_indices("r$") {
            let tail = &normalized[position + 2..];
            let amount = tail.trim_start();
            let length = amount
                .bytes()
                .take_while(|byte| byte.is_ascii_digit() || *byte == b'.' || *byte == b',')
                .count();
            let prefix = normalized[..position].trim_end();
            let suffix = &amount[length..];
            if is_excluded_price(prefix, suffix) {
                continue;
            }

            if let Some(cents) = parse_brl(&amount[..length]) {
                prices.push(Price { cents, line });
            }
        }
    }
    prices
}

fn is_excluded_price(prefix: &str, suffix: &str) -> bool {
    let words: Vec<_> = prefix.split_whitespace().collect();
    let last = words
        .last()
        .copied()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric());
    let installment = last
        .strip_suffix('x')
        .is_some_and(|count| !count.is_empty() && count.chars().all(|c| c.is_ascii_digit()));
    let spaced_installment = last == "x"
        && words
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|count| count.chars().all(|c| c.is_ascii_digit()));
    let installment_words = ["parcelas", "parcela", "mensal", "mensais", "mensalidade"];
    let suffix = suffix.trim_start();

    installment
        || spaced_installment
        || last == "de"
        || installment_words.contains(&last)
        || last == "frete"
        || prefix.ends_with("x de")
        || prefix.ends_with("parcelas de")
        || suffix.starts_with("/mês")
        || suffix.starts_with("/mes")
        || suffix.starts_with("por mês")
        || suffix.starts_with("por mes")
        || suffix.starts_with("por parcela")
        || suffix.starts_with("/parcela")
        || suffix.starts_with("mensais")
        || suffix.starts_with("mensal")
        || suffix.starts_with("de frete")
        || suffix.strip_prefix("em ").is_some_and(starts_installments)
}

fn starts_installments(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let first = words.next().unwrap_or("");
    let count = first.strip_suffix('x').unwrap_or(first);
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }

    first.ends_with('x') || matches!(words.next(), Some("x" | "parcelas"))
}

fn parse_brl(amount: &str) -> Option<i64> {
    let amount = amount.trim_end_matches('.');
    let (whole, fraction) = match amount.split_once(',') {
        Some((whole, fraction))
            if fraction.len() == 2 && fraction.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            (whole, fraction.parse::<i64>().ok()?)
        }
        Some(_) => return None,
        None => (amount, 0),
    };
    let groups: Vec<_> = whole.split('.').collect();
    if groups.len() > 1
        && (groups[0].is_empty()
            || groups[0].len() > 3
            || groups.iter().skip(1).any(|group| group.len() != 3))
    {
        return None;
    }
    let digits = groups.concat();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let cents = digits
        .parse::<i64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(fraction)?;
    (cents > 0).then_some(cents)
}
