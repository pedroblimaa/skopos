use grammers_client::tl;

pub(in crate::telegram) fn photo_media(cached: bool, bytes: Vec<u8>) -> tl::enums::MessageMedia {
    let size = if cached {
        tl::types::PhotoCachedSize {
            r#type: "x".into(),
            w: 32,
            h: 32,
            bytes,
        }
        .into()
    } else {
        tl::types::PhotoSize {
            r#type: "x".into(),
            w: 32,
            h: 32,
            size: 4096,
        }
        .into()
    };
    tl::types::MessageMediaPhoto {
        spoiler: false,
        photo: Some(
            tl::types::Photo {
                has_stickers: false,
                id: 42,
                access_hash: 24,
                file_reference: vec![1],
                date: 100,
                sizes: vec![size],
                video_sizes: None,
                dc_id: 2,
            }
            .into(),
        ),
        ttl_seconds: None,
        live_photo: false,
        video: None,
    }
    .into()
}

pub(in crate::telegram) fn message(
    peer: tl::enums::Peer,
    id: i32,
    date: i32,
    text: &str,
) -> tl::enums::Message {
    tl::types::Message {
        out: false,
        mentioned: false,
        media_unread: false,
        silent: false,
        post: false,
        from_scheduled: false,
        legacy: false,
        edit_hide: false,
        pinned: false,
        noforwards: false,
        invert_media: false,
        offline: false,
        video_processing_pending: false,
        paid_suggested_post_stars: false,
        paid_suggested_post_ton: false,
        id,
        from_id: None,
        from_boosts_applied: None,
        from_rank: None,
        peer_id: peer,
        saved_peer_id: None,
        fwd_from: None,
        via_bot_id: None,
        via_business_bot_id: None,
        guestchat_via_from: None,
        reply_to: None,
        date,
        message: text.into(),
        media: None,
        reply_markup: None,
        entities: None,
        views: None,
        forwards: None,
        replies: None,
        edit_date: None,
        post_author: None,
        grouped_id: None,
        reactions: None,
        restriction_reason: None,
        ttl_period: None,
        quick_reply_shortcut_id: None,
        effect: None,
        factcheck: None,
        report_delivery_until_date: None,
        paid_message_stars: None,
        suggested_post: None,
        schedule_repeat_period: None,
        summary_from_language: None,
        rich_message: None,
    }
    .into()
}

pub(in crate::telegram) fn page(
    messages: Vec<tl::enums::Message>,
) -> tl::enums::messages::Messages {
    tl::types::messages::Messages {
        messages,
        topics: vec![],
        chats: vec![],
        users: vec![],
    }
    .into()
}
