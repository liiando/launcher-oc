// Windows GUI app: never allocate a console window (debug or release).
// `not(test)` keeps `cargo test` output visible on the console.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod api;
mod fingerprint;
mod license;
mod protect;
mod style;
mod updater;

use iced::{
    alignment::Horizontal,
    executor,
    keyboard::key,
    keyboard::on_key_press,
    keyboard::Key,
    widget::{button, column, container, row, text, text_input, Space},
    Alignment, Application, Command, Element, Length, Settings, Size, Subscription, Theme,
};

use api::License;

pub fn main() -> iced::Result {
    // Hidden CLI hooks for headless update testing. The release build is a GUI
    // subsystem app (no stdout attached), so results are written to temp files.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        let _ = std::fs::write(
            std::env::temp_dir().join("oc_launcher_version.txt"),
            env!("CARGO_PKG_VERSION"),
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--self-update") {
        let result = updater::cli_self_update();
        let _ = std::fs::write(
            std::env::temp_dir().join("oc_launcher_update.txt"),
            format!("{result:?}"),
        );
        return Ok(());
    }

    let settings = Settings {
        window: iced::window::Settings {
            size: Size::new(460.0, 880.0),
            resizable: false,
            icon: iced::window::icon::from_file_data(include_bytes!("../assets/icon.png"), None)
                .ok(),
            ..Default::default()
        },
        fonts: vec![
            include_bytes!("../fonts/BebasNeue-Regular.ttf")
                .as_slice()
                .into(),
            include_bytes!("../fonts/ChakraPetch-Regular.ttf")
                .as_slice()
                .into(),
        ],
        default_font: style::BODY,
        antialiasing: true,
        ..Settings::default()
    };
    OnlyClimb::run(settings)
}

struct OnlyClimb {
    fingerprint: String,
    license_key: String,
    tiktok_username: String,
    status: String,
    status_ok: bool,
    license_valid: bool,
    is_checking: bool,
    api_response: Option<License>,
    update: updater::UpdateState,
}

#[derive(Debug, Clone)]
enum Message {
    LicenseKeyChanged(String),
    TikTokUserChanged(String),
    CheckLicense,
    LicenseResult(Box<Result<api::LicenseResponse, String>>),
    LaunchGame,
    KeyPress(Key),
    UpdateChecked(updater::UpdateState),
    UpdateApply,
    UpdateApplied(updater::UpdateState),
}

impl Application for OnlyClimb {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        protect::encrypt_if_needed();
        let fp = fingerprint::generate_fingerprint();

        let saved = license::read_license();

        (
            Self {
                fingerprint: fp,
                license_key: saved
                    .as_ref()
                    .map(|l| l.license_key.clone())
                    .unwrap_or_default(),
                tiktok_username: saved
                    .as_ref()
                    .map(|l| l.tiktok_username.clone())
                    .unwrap_or_default(),
                status: if saved.is_some() {
                    String::from("Licence trouvée. Cliquez sur Activer pour vérifier.")
                } else {
                    String::from("Entrez votre clé de licence et votre TikTok pour activer.")
                },
                status_ok: true,
                license_valid: false,
                is_checking: false,
                api_response: None,
                update: updater::UpdateState::Idle,
            },
            Command::perform(updater::check(), Message::UpdateChecked),
        )
    }

    fn title(&self) -> String {
        String::from("Only Climb Together")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LicenseKeyChanged(input) => {
                self.license_key = input;
            }
            Message::TikTokUserChanged(input) => {
                self.tiktok_username = input;
            }
            Message::KeyPress(key) => {
                if key == Key::Named(key::Named::Enter) {
                    return self.begin_check();
                }
            }
            Message::CheckLicense => {
                return self.begin_check();
            }
            Message::LicenseResult(result) => {
                self.is_checking = false;
                match *result {
                    Ok(resp) => {
                        if resp.valid {
                            self.status = if resp.message.is_empty() {
                                String::from("Licence valide ! Vous pouvez lancer le jeu.")
                            } else {
                                resp.message.clone()
                            };
                            self.status_ok = true;
                            self.license_valid = true;
                            self.api_response = resp.license.clone().or_else(|| {
                                Some(License {
                                    tiktok_username: self.tiktok_username.clone(),
                                    ..Default::default()
                                })
                            });
                            if let Err(e) = license::save_license(
                                &self.license_key,
                                &self.fingerprint,
                                &self.tiktok_username,
                            ) {
                                self.status = e;
                                self.status_ok = false;
                            }
                        } else {
                            self.status = if !resp.message.is_empty() {
                                resp.message.clone()
                            } else if !resp.reason.is_empty() {
                                format!("Licence invalide ({})", resp.reason)
                            } else {
                                String::from("Licence invalide: clé refusée par le serveur.")
                            };
                            self.status_ok = false;
                            self.license_valid = false;
                            // Keep the detail block when the server returns one
                            // (e.g. revoked / expired / fingerprint_mismatch) so the
                            // user can see why; it is absent for unknown keys.
                            self.api_response = resp.license.clone();
                        }
                    }
                    Err(e) => {
                        self.status = format!("Erreur réseau: {e}");
                        self.status_ok = false;
                        self.license_valid = false;
                    }
                }
            }
            Message::LaunchGame => {
                protect::decrypt_and_launch();
            }
            Message::UpdateChecked(state) => {
                // A failed *startup* check (e.g. offline) stays silent.
                self.update = match state {
                    updater::UpdateState::Error(_) => updater::UpdateState::Idle,
                    s => s,
                };
            }
            Message::UpdateApply => {
                self.update = updater::UpdateState::Installing;
                return Command::perform(updater::apply(true), Message::UpdateApplied);
            }
            Message::UpdateApplied(state) => {
                // On success the new binary has been launched; exit to free the
                // file for the swap. Otherwise surface the resulting state.
                if matches!(state, updater::UpdateState::ReadyToQuit) {
                    std::process::exit(0);
                }
                self.update = state;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let eyebrow = text("LICENCE · ACCÈS AU SOMMET")
            .font(style::DISPLAY)
            .size(15)
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center)
            .style(style::CYAN);

        let wordmark = column![
            text("ONLY CLIMB")
                .font(style::DISPLAY)
                .size(64)
                .line_height(0.84)
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .style(style::TEXT),
            text("TOGETHER")
                .font(style::DISPLAY)
                .size(64)
                .line_height(0.84)
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .style(style::CYAN),
        ]
        .width(Length::Fill);

        let subtitle = text("Gestionnaire de licence")
            .size(13)
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center)
            .style(style::MUTED);

        let divider = container(Space::with_height(1.0))
            .width(Length::Fill)
            .style(style::divider());

        let fp_chip = container(
            column![
                text("APPAREIL").size(11).style(style::MUTED),
                text(&self.fingerprint).size(15).style(style::CYAN),
            ]
            .spacing(3),
        )
        .width(Length::Fill)
        .padding([10, 14])
        .style(style::inset());

        let key_block = column![
            text("CLÉ DE LICENCE").size(11).style(style::MUTED),
            text_input("XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX", &self.license_key)
                .on_input(Message::LicenseKeyChanged)
                .on_submit(Message::CheckLicense)
                .size(15)
                .padding([12, 14])
                .style(style::field()),
        ]
        .width(Length::Fill)
        .spacing(7);

        let tiktok_block = column![
            text("COMPTE TIKTOK").size(11).style(style::MUTED),
            text_input("@votre_compte", &self.tiktok_username)
                .on_input(Message::TikTokUserChanged)
                .on_submit(Message::CheckLicense)
                .size(15)
                .padding([12, 14])
                .style(style::field()),
        ]
        .width(Length::Fill)
        .spacing(7);

        let mut activate_btn = button(
            text(if self.is_checking {
                "VÉRIFICATION…"
            } else {
                "ACTIVER"
            })
            .font(style::DISPLAY)
            .size(21)
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(13)
        .style(style::activate());

        if !self.is_checking {
            activate_btn = activate_btn.on_press(Message::CheckLicense);
        }

        let status_pill = container(text(&self.status).size(12.5).style(if self.status_ok {
            style::CYAN
        } else {
            style::MAGENTA
        }))
        .width(Length::Fill)
        .padding([9, 13])
        .style(style::pill(self.status_ok));

        let mut launch_btn = button(
            text("LANCER LE JEU")
                .font(style::DISPLAY)
                .size(24)
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(15)
        .style(style::launch());

        if self.license_valid {
            launch_btn = launch_btn.on_press(Message::LaunchGame);
        }

        let update_banner: Element<Message> = match &self.update {
            updater::UpdateState::Available(v) => container(
                row![
                    text(format!("Mise à jour {v} disponible"))
                        .size(12.5)
                        .width(Length::Fill)
                        .style(style::CYAN),
                    button(text("INSTALLER").font(style::DISPLAY).size(14))
                        .padding([6, 14])
                        .style(style::activate())
                        .on_press(Message::UpdateApply),
                ]
                .align_items(Alignment::Center)
                .spacing(10),
            )
            .width(Length::Fill)
            .padding([9, 12])
            .style(style::inset())
            .into(),
            updater::UpdateState::Installing => {
                container(text("Mise à jour en cours…").size(12.5).style(style::CYAN))
                    .width(Length::Fill)
                    .padding([9, 12])
                    .style(style::inset())
                    .into()
            }
            updater::UpdateState::Error(e) => container(
                text(format!("Échec de la mise à jour : {e}"))
                    .size(12)
                    .style(style::MAGENTA),
            )
            .width(Length::Fill)
            .padding([9, 12])
            .style(style::inset())
            .into(),
            _ => Space::with_height(0.0).into(),
        };
        let banner_gap: Element<Message> = if matches!(
            self.update,
            updater::UpdateState::Available(_)
                | updater::UpdateState::Installing
                | updater::UpdateState::Error(_)
        ) {
            Space::with_height(16.0).into()
        } else {
            Space::with_height(0.0).into()
        };

        let mut card_inner = column![
            update_banner,
            banner_gap,
            eyebrow,
            Space::with_height(12.0),
            wordmark,
            Space::with_height(6.0),
            subtitle,
            Space::with_height(16.0),
            divider,
            Space::with_height(16.0),
            fp_chip,
            Space::with_height(16.0),
            key_block,
            Space::with_height(13.0),
            tiktok_block,
            Space::with_height(16.0),
            activate_btn,
            Space::with_height(12.0),
            status_pill,
        ]
        .width(Length::Fill)
        .align_items(Alignment::Center);

        if let Some(info) = self.api_response.as_ref() {
            let mut rows = column![text("INFORMATIONS LICENCE")
                .font(style::DISPLAY)
                .size(15)
                .style(style::CYAN)]
            .spacing(6);

            if !info.tiktok_username.is_empty() {
                rows = rows.push(info_row("Utilisateur", info.tiktok_username.clone()));
            }
            if !info.status.is_empty() {
                rows = rows.push(info_row(
                    "Statut",
                    license_status_fr(&info.status).to_string(),
                ));
            }
            if !info.created_at.is_empty() {
                rows = rows.push(info_row("Créée le", info.created_at.clone()));
            }
            if !info.status_changed_at.is_empty() {
                rows = rows.push(info_row("Modifiée le", info.status_changed_at.clone()));
            }
            if !info.expires_at.is_empty() {
                let v = match info.expires_in_days {
                    Some(d) if d >= 0 => format!("{} · {} j", info.expires_at, d),
                    Some(d) => format!("{} · expiré +{} j", info.expires_at, -d),
                    None => info.expires_at.clone(),
                };
                rows = rows.push(info_row("Expire", v));
            }
            if !info.last_check.is_empty() {
                rows = rows.push(info_row("Vérifié", info.last_check.clone()));
            }
            if info.fingerprint_bound && !info.fingerprint_match {
                rows = rows.push(
                    text("⚠ Licence liée à un autre appareil")
                        .size(12)
                        .style(style::MAGENTA),
                );
            }

            let panel = container(rows)
                .width(Length::Fill)
                .padding([14, 16])
                .style(style::inset());

            card_inner = card_inner.push(Space::with_height(14.0)).push(panel);
        }

        card_inner = card_inner.push(Space::with_height(16.0)).push(launch_btn);

        let card = container(card_inner)
            .width(Length::Fixed(404.0))
            .padding(28)
            .style(style::card());

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(16)
            .style(style::root())
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        on_key_press(|key, _modifiers| Some(Message::KeyPress(key)))
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

/// Maps the server's stable license status code to a French label for display.
fn license_status_fr(status: &str) -> &str {
    match status {
        "active" => "Active",
        "revoked" => "Révoquée",
        "expired" => "Expirée",
        "inactive" => "Désactivée",
        other => other,
    }
}

/// A key/value line for the license info panel: muted label left, value right.
fn info_row<'a>(key: &str, value: String) -> Element<'a, Message> {
    row![
        text(key.to_string()).size(12).style(style::MUTED),
        Space::with_width(Length::Fill),
        text(value).size(12).style(style::TEXT),
    ]
    .align_items(Alignment::Center)
    .into()
}

impl OnlyClimb {
    /// Shared entry point for both the "Activer" button and the Enter key:
    /// validates the inputs, sets the checking state, and fires the request.
    /// No-op while a check is already in flight.
    fn begin_check(&mut self) -> Command<Message> {
        if self.is_checking {
            return Command::none();
        }
        if self.license_key.trim().is_empty() {
            self.status = String::from("Veuillez entrer une clé de licence.");
            self.status_ok = false;
            return Command::none();
        }
        if self.tiktok_username.trim().is_empty() {
            self.status = String::from("Le nom d'utilisateur TikTok est obligatoire.");
            self.status_ok = false;
            return Command::none();
        }
        self.is_checking = true;
        self.status = String::from("Vérification en cours...");
        self.status_ok = true;
        self.check_license()
    }

    fn check_license(&self) -> Command<Message> {
        let key = self.license_key.trim().to_string();
        let fp = self.fingerprint.clone();
        let tiktok = self.tiktok_username.trim().to_string();
        Command::perform(
            async move { api::validate_license(&key, &fp, &tiktok).await },
            |result| Message::LicenseResult(Box::new(result)),
        )
    }
}
