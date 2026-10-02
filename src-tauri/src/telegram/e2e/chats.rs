use grammers_client::tl;

pub(in crate::telegram) fn account(id: i64) -> tl::types::User {
    tl::types::User {
        is_self: true,
        contact: false,
        mutual_contact: false,
        deleted: false,
        bot: false,
        bot_chat_history: false,
        bot_nochats: false,
        verified: false,
        restricted: false,
        min: false,
        bot_inline_geo: false,
        support: false,
        scam: false,
        apply_min_photo: false,
        fake: false,
        bot_attach_menu: false,
        premium: false,
        attach_menu_enabled: false,
        bot_can_edit: false,
        close_friend: false,
        stories_hidden: false,
        stories_unavailable: false,
        contact_require_premium: false,
        bot_business: false,
        bot_has_main_app: false,
        bot_forum_view: false,
        bot_forum_can_manage_topics: false,
        bot_can_manage_bots: false,
        bot_guestchat: false,
        bot_guard: false,
        id,
        access_hash: None,
        first_name: Some("Fixture".into()),
        last_name: None,
        username: None,
        phone: None,
        photo: None,
        status: None,
        bot_info_version: None,
        restriction_reason: None,
        bot_inline_placeholder: None,
        lang_code: None,
        emoji_status: None,
        usernames: None,
        stories_max_id: None,
        color: None,
        profile_color: None,
        bot_active_users: None,
        bot_verification_icon: None,
        send_paid_messages_stars: None,
    }
}

pub(in crate::telegram) fn group(id: i64) -> tl::types::Chat {
    tl::types::Chat {
        creator: false,
        left: false,
        deactivated: false,
        call_active: false,
        call_not_empty: false,
        noforwards: false,
        id,
        title: format!("Group {id}"),
        photo: tl::enums::ChatPhoto::Empty,
        participants_count: 0,
        date: 0,
        version: 0,
        migrated_to: None,
        admin_rights: None,
        default_banned_rights: None,
    }
}

pub(in crate::telegram) fn channel(id: i64) -> tl::types::Channel {
    tl::types::Channel {
        creator: false,
        left: false,
        broadcast: true,
        verified: false,
        megagroup: false,
        restricted: false,
        signatures: false,
        min: false,
        scam: false,
        has_link: false,
        has_geo: false,
        slowmode_enabled: false,
        call_active: false,
        call_not_empty: false,
        fake: false,
        gigagroup: false,
        noforwards: false,
        join_to_send: false,
        join_request: false,
        forum: false,
        stories_hidden: false,
        stories_hidden_min: false,
        stories_unavailable: false,
        signature_profiles: false,
        autotranslation: false,
        broadcast_messages_allowed: false,
        monoforum: false,
        forum_tabs: false,
        id,
        access_hash: Some(42),
        title: format!("Channel {id}"),
        username: Some(format!("channel{id}")),
        photo: tl::enums::ChatPhoto::Empty,
        date: 0,
        restriction_reason: None,
        admin_rights: None,
        banned_rights: None,
        default_banned_rights: None,
        participants_count: None,
        usernames: None,
        stories_max_id: None,
        color: None,
        profile_color: None,
        emoji_status: None,
        level: None,
        subscription_until_date: None,
        bot_verification_icon: None,
        send_paid_messages_stars: None,
        linked_monoforum_id: None,
    }
}

pub(in crate::telegram) fn dialog(peer: tl::enums::Peer) -> tl::enums::Dialog {
    tl::types::Dialog {
        pinned: false,
        unread_mark: false,
        view_forum_as_messages: false,
        peer,
        top_message: 10,
        read_inbox_max_id: 0,
        read_outbox_max_id: 0,
        unread_count: 0,
        unread_mentions_count: 0,
        unread_reactions_count: 0,
        unread_poll_votes_count: 0,
        notify_settings: notify_settings(),
        pts: None,
        draft: None,
        folder_id: None,
        ttl_period: None,
    }
    .into()
}

fn notify_settings() -> tl::enums::PeerNotifySettings {
    tl::types::PeerNotifySettings {
        show_previews: None,
        silent: None,
        mute_until: None,
        ios_sound: None,
        android_sound: None,
        other_sound: None,
        stories_muted: None,
        stories_hide_sender: None,
        stories_ios_sound: None,
        stories_android_sound: None,
        stories_other_sound: None,
    }
    .into()
}

#[cfg(test)]
pub(in crate::telegram) fn message(peer: tl::enums::Peer) -> tl::enums::Message {
    tl::types::MessageService {
        out: false,
        mentioned: false,
        media_unread: false,
        reactions_are_possible: false,
        silent: false,
        post: false,
        legacy: false,
        id: 10,
        from_id: None,
        peer_id: peer,
        saved_peer_id: None,
        reply_to: None,
        date: 1234,
        action: tl::enums::MessageAction::Empty,
        reactions: None,
        ttl_period: None,
    }
    .into()
}

pub(in crate::telegram) fn page(archived: bool, with_photos: bool) -> tl::enums::messages::Dialogs {
    let photo = if with_photos {
        tl::types::ChatPhoto {
            has_video: false,
            photo_id: 42,
            stripped_thumb: None,
            dc_id: 2,
        }
        .into()
    } else {
        tl::enums::ChatPhoto::Empty
    };
    let (peer, chat): (tl::enums::Peer, tl::enums::Chat) = if archived {
        let mut channel = channel(2);
        channel.photo = photo;

        (
            tl::types::PeerChannel { channel_id: 2 }.into(),
            channel.into(),
        )
    } else {
        let mut group = group(1);
        group.photo = photo;

        (tl::types::PeerChat { chat_id: 1 }.into(), group.into())
    };

    tl::types::messages::Dialogs {
        dialogs: vec![dialog(peer)],
        messages: vec![],
        chats: vec![chat],
        users: vec![],
    }
    .into()
}
