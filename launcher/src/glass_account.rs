//! Settings' Account card in the glass look, as the mockup's: who plays now (a big head, the
//! name, how), and the offline names as chips with a field to add one. The offline build has
//! no Microsoft card: the account is the whole card, at the full width.
//!
//! Not in the mockup, kept in its patterns: the name the game made up (a chip with
//! New name), each offline name's skin (a pencil in its chip) and the skin panel.

use super::*;
use crate::glass_ui::{self as ui, white, Tone};
use crate::servers_page::Field;
use account::Player;
use horizon_theme::glass::{fill, ink, radius};

impl Launcher {
    /// The account card: the whole of it is who plays now — the player, the offline names and
    /// the field to add one. The offline build has no Microsoft card beside them.
    pub(super) fn glass_account(&mut self, cx: &mut Context<Self>) -> Div {
        self.glass_who_plays(cx)
            .flex_1()
            .min_w_0()
            .p(px(20.))
    }

    /// The player now, the offline names and the field to add one.
    fn glass_who_plays(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let now = self.accounts.player();
        let (uuid, name, status) = match &now {
            Player::Microsoft(uuid) => (
                Some(uuid.clone()),
                self.accounts
                    .accounts
                    .iter()
                    .find(|a| &a.uuid == uuid)
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                (Tone::Live, tr("Microsoft account · online servers too")),
            ),
            Player::Offline(name) => (
                None,
                name.clone(),
                (Tone::Off, tr("Offline · single player and LAN worlds")),
            ),
            Player::GameName => (
                None,
                self.offline_name.clone().unwrap_or_else(|| "Player".into()),
                (Tone::Off, tr("Offline · single player and LAN worlds")),
            ),
        };
        let head = div()
            .flex_none()
            .size(px(56.))
            .rounded(px(14.))
            .overflow_hidden()
            .child(
                self.profile_heads
                    .draw(uuid.as_deref(), 56., self.scale, &t),
            );
        let header = div().flex().items_center().gap(px(16.)).child(head).child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap(px(10.))
                .child(
                    div()
                        .font_family(horizon_theme::glass::DISPLAY_FONT)
                        .font_weight(FontWeight::SEMIBOLD)
                        // A compact summary: the top bar and the account rows name the
                        // player too, so this is no page headline.
                        .text_size(px(22.))
                        .line_height(px(28.))
                        .text_color(white(ink::FULL))
                        .truncate()
                        .child(name),
                )
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8.))
                        .child(ui::dot(status.0).mt(px(7.)))
                        .child(
                            ui::text(14., FontWeight::NORMAL, 0.65)
                                .flex_1()
                                .min_w_0()
                                .line_height(px(21.))
                                .child(status.1),
                        ),
                ),
        );

        // The chips: each offline name, the game's own name, then the field.
        let mut chips = div().flex().flex_wrap().gap(px(8.)).mt(px(12.));
        for (i, offline) in self.accounts.offline.clone().into_iter().enumerate() {
            let plays = now == Player::Offline(offline.clone());
            let (who, skin, forget) = (offline.clone(), offline.clone(), offline.clone());
            chips = chips.child(
                self.name_chip(
                    SharedString::from(format!("offline-chip-{i}")),
                    offline,
                    plays,
                )
                .when(!plays, |d| {
                    d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.play_as(Player::Offline(who.clone()));
                        cx.notify();
                    }))
                })
                .child(self.chip_icon(
                    SharedString::from(format!("offline-skin-{i}")),
                    "pencil-edit-02",
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.open_skin(skin.clone());
                        cx.notify();
                    }),
                ))
                .when(!plays, |d| {
                    d.child(self.chip_icon(
                        SharedString::from(format!("offline-forget-{i}")),
                        "cancel-01",
                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.forget_offline(&forget);
                            cx.notify();
                        }),
                    ))
                }),
            );
        }
        if let Some(made_up) = self.offline_name.clone() {
            let plays = now == Player::GameName;
            chips = chips.child(
                self.name_chip("game-name-chip", made_up, plays)
                    .when(!plays, |d| {
                        d.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.play_as(Player::GameName);
                            cx.notify();
                        }))
                    })
                    .child(self.chip_icon(
                        "game-name-new".into(),
                        "refresh",
                        cx.listener(|this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            match instances::new_player_name() {
                                Ok(name) => this.offline_name = Some(name),
                                Err(e) => this.sign_in_state = Some(SignInState::Done(Err(e))),
                            }
                            cx.notify();
                        }),
                    )),
            );
        }
        let add = div()
            .flex()
            .items_center()
            .h(px(44.))
            .pr(px(6.))
            .rounded(px(radius::ROW))
            .bg(white(0.05))
            .child(self.field(Field::OfflineName, tr("Add a name"), 150., cx))
            .child(
                ui::quiet("offline-add", tr("Add"), 32., &t)
                    .px(px(12.))
                    .text_size(px(13.))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.offline_name_enter();
                        cx.notify();
                    })),
            );
        chips = chips.child(add);
        div()
            .flex()
            .flex_col()
            .child(header)
            .child(
                ui::text(13., FontWeight::MEDIUM, 0.6)
                    .mt(px(20.))
                    .child(tr("Offline names")),
            )
            .child(chips)
            .child(
                ui::text(13., FontWeight::NORMAL, 0.5)
                    .mt(px(10.))
                    .line_height(px(19.5))
                    .child(tr(
                        "3 to 16 letters, numbers or underscores. Switch names from the top bar \
                         too. Online servers don't take offline names.",
                    )),
            )
            .children(self.skin_panel(cx).map(|p| p.mt(px(12.))))
    }

    /// An offline name's chip: its head and name, a tick on the one playing; white at 14 % for
    /// that one, 5 % (8 % under the pointer) for the others.
    fn name_chip(&self, id: impl Into<SharedString>, name: String, plays: bool) -> Stateful<Div> {
        let t = self.theme;
        div()
            .id(id.into())
            .flex()
            .items_center()
            .gap(px(10.))
            .h(px(44.))
            .pl(px(6.))
            .pr(px(6.))
            .rounded(px(radius::ROW))
            .when(plays, |d| d.bg(white(0.14)))
            .when(!plays, |d| {
                d.bg(white(0.05))
                    .cursor_pointer()
                    .hover(|s| s.bg(white(0.08)))
            })
            .child(
                div()
                    .flex_none()
                    .size(px(30.))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .child(self.profile_heads.draw(None, 30., self.scale, &t)),
            )
            .child(ui::text(14., FontWeight::MEDIUM, ink::FULL).child(name))
            .when(plays, |d| {
                d.child(ui::icon("tick-02", 16., ui::color(ui::look().accent_soft)))
            })
    }

    /// A 28 px button in a chip (`size-7 rounded-[8px]`), white at 45 %.
    fn chip_icon(
        &self,
        id: SharedString,
        icon: &'static str,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Stateful<Div> {
        keyboard(
            div()
                .id(id)
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(28.))
                .rounded(px(8.))
                .cursor_pointer()
                .hover(|s| s.bg(white(fill::BUTTON)))
                .child(ui::icon(icon, 14., white(0.45))),
            &self.theme,
        )
        .on_click(on_click)
    }
}
