# Saved Messages and desktop alerts

Skopos saves matching promotions to **Saved Messages / Mensagens Salvas** in the Telegram account already connected to the app. No bot, token, or additional login is required.

## Use it

1. Connect your Telegram account, select promotion chats, and add products.
2. Open your profile menu, then **Notifications / Notificações**. **Save to Telegram / Salvar no Telegram** and **Desktop alerts / Alertas no computador** are both initially enabled and can be changed independently.
3. Run **Search now / Buscar agora**.
4. Open **Saved Messages / Mensagens Salvas** in Telegram to see each new matching promotion, with a short description, price, clickable offer link, and available image. Skopos also requests one desktop summary.

Messages sync through Telegram across your devices. Saving a message to yourself does not guarantee a phone notification. Skopos must be running. Automatic monitoring continues when the window is hidden in the tray; see [automatic monitoring](monitoring.md) for the schedule and Windows startup settings.

Before the first promotion of each local search day, Skopos sends one separate text message with a bold date divider. Promotion captions contain only the offer details. Later searches and app restarts keep that day's divider history per account. An uncertain divider pauses Telegram delivery until you check Saved Messages and explicitly confirm retry; a confirmed retry can duplicate it.

## Delivery and troubleshooting

- Reposts are suppressed when the product's price and offer URL are unchanged from its last notification. Changing either makes a new notification eligible. Telegram message links are not used as offer identity.
- Successful deliveries are recorded separately from saved search results. Another search, reopening the page, or clearing results does not resend them.
- Delivery failures do not remove search results. Definitely unsent promotions are retried while Skopos runs, subject to cooldowns and current product criteria.
- Uncertain saves are not retried automatically: Telegram may already have received them. Check Saved Messages before using the confirmed retry action in Notifications.
- Telegram rate limits preserve a cooldown before another delivery attempt.
- Settings and delivery history stay local and separate for each Telegram account. Existing switch preferences and successful bot deliveries are preserved when switching to Saved Messages.
- Native Windows notification appearance requires verification with an installed build. System settings and Do Not Disturb can suppress visible alerts.

Reference: [Telegram Saved Messages](https://core.telegram.org/api/saved-messages).
