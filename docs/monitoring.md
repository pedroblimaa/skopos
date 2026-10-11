# Automatic monitoring

Open the Telegram profile menu and choose **Monitoring / Monitoramento**. Automatic monitoring and Start with Windows initially default on and have separate switches. Monitoring preferences belong to the connected Telegram account; startup belongs to this installation.

Skopos searches at its first eligible start each local day, then at or after **18:00**. If the computer is off at 18:00, the evening search runs when Skopos next starts that day. If the first start is after 18:00, one catch-up search uses both daily slots. An app left running also starts a new day's first search after midnight. Sleep/resume is checked within a minute.

Daily slots are saved before history requests. Failed or interrupted searches count toward the limit. A missing login, product, or selected chat does not consume a slot. Neither does waiting for another search or cleanup. Use **Search now** to retry manually; it always checks the last 24 hours and does not consume automatic slots.

Automatic searches initially check the last 24 hours of each selected chat. Later searches continue after that chat's last successfully checked message, including messages accumulated while the computer was off. Failed chats keep their previous checkpoints. Clearing saved results does not clear these checkpoints or notification history.

Closing the window hides it in the system tray. **Open Skopos / Abrir Skopos** restores it; **Quit / Sair** stops monitoring and exits. Windows startup opens Skopos hidden in the tray. Launching Skopos again shows the existing instance. A tray failure leaves ordinary close behavior available.

Windows startup registration is enabled only in release builds, and never in E2E builds. Development builds let you review the switches and monitoring behavior without changing the Windows startup registration.

Matching, saved results, Saved Messages delivery, and desktop summaries use the existing flow. Reposts with the same product price and offer URL are suppressed separately for each notification channel. Uncertain Telegram deliveries still require explicit retry confirmation. See [notifications](notification-setup.md).

## Reviewing this implementation

Run `pnpm tauri dev`, connect Telegram, and save products and chats. Check Monitoring in the profile menu, automatic search status/results, and manual Search now. Close the window and reopen it through the tray. Use Quit to exit.

The new desktop dependencies and sign-out/quit lifecycle changes need human review. Native Windows startup, tray activation, and notification appearance require a packaged release review.

## Overnight release trial

1. Quit any development copy through its tray menu, install the release, and open Skopos.
2. Log in, save at least one product and selected chat, and enable both switches in Monitoring. Check the notification switches too.
3. Close the window and verify that the tray icon remains. Use Open to restore the window.
4. Reboot before 18:00 tomorrow. Skopos should start hidden in the tray. Open it and check that Last automatic attempt belongs to tomorrow.
5. Keep the computer awake with Skopos running through 18:00. Closing the window is fine; Quit stops the app. Check Last automatic attempt again after one minute.
6. Check saved results, Telegram Saved Messages, and desktop notifications. No notification is expected when there are no new qualifying offers or the last price and offer link are unchanged.

An app left running across midnight consumes the next day's first slot then, so a later morning reboot will not repeat that slot. Failed attempts count; inspect the displayed error and use Search now for a manual retry.

## Local verification

On 6 October 2026, local formatting/lint/types and 342 frontend unit tests passed. All 121 native unit tests and production/E2E Clippy checks passed. The final desktop binary passed 23 scenarios across monitoring, notifications, and manual search (51 seconds). Windows startup and tray operation still require the installed-release trial above. Full CI coverage, authentication desktop scenarios, and complete production verification remain pending; the 96% coverage thresholds are unchanged.
