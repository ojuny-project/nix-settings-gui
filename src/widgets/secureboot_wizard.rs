//! Guided Secure Boot setup. The dialog always derives the next stage from live
//! state, so it can be reopened after a reboot or a firmware change.

use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::context::BuildContext;
use super::secureboot::{badge, StatusView};
use crate::model::Record;
use crate::secureboot::{self, KeyAction, SecureBootState, SetupOptions, SetupStage};

const STAGES: [SetupStage; 8] = [
    SetupStage::Keys,
    SetupStage::Config,
    SetupStage::Rebuild,
    SetupStage::ClearKeys,
    SetupStage::Enroll,
    SetupStage::EnableSecureBoot,
    SetupStage::Verify,
    SetupStage::Done,
];

fn key_action(index: u32) -> KeyAction {
    match index {
        1 => KeyAction::Create,
        2 => KeyAction::Migrate,
        _ => KeyAction::Keep,
    }
}

fn stage_position(stage: SetupStage) -> usize {
    STAGES.iter().position(|item| *item == stage).unwrap_or(0)
}

pub(crate) fn open(
    parent: Option<&gtk::Window>,
    record: Record,
    ctx: &BuildContext,
    configured: bool,
    on_done: Rc<dyn Fn()>,
) {
    let dialog = gtk::Dialog::builder()
        .title("Secure Boot setup")
        .modal(true)
        .default_width(760)
        .default_height(780)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }
    dialog.add_button("Close", gtk::ResponseType::Close);

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    scrolled.set_child(Some(&content));
    dialog.content_area().append(&scrolled);

    let stage_title = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .css_classes(["title-2"])
        .label("Checking state...")
        .build();
    content.append(&stage_title);
    let stage_detail = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    content.append(&stage_detail);

    let checklist = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let mut stage_rows: Vec<(SetupStage, gtk::Image, gtk::Label)> = Vec::new();
    for stage in STAGES {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let icon = gtk::Image::from_icon_name("radio-symbolic");
        icon.set_pixel_size(16);
        let label = gtk::Label::builder()
            .label(stage.title())
            .xalign(0.0)
            .css_classes(["caption"])
            .build();
        row.append(&icon);
        row.append(&label);
        checklist.append(&row);
        stage_rows.push((stage, icon, label));
    }
    content.append(&checklist);

    let status = StatusView::new();

    let options_group = adw::PreferencesGroup::new();
    options_group.set_title("Setup options");

    let key_drop = gtk::DropDown::from_strings(&[
        KeyAction::Keep.label(),
        KeyAction::Create.label(),
        KeyAction::Migrate.label(),
    ]);
    key_drop.set_selected(0);
    let key_row = adw::ActionRow::new();
    key_row.set_title("Keys");
    key_row.set_subtitle("Existing keys are never overwritten");
    key_drop.set_valign(gtk::Align::Center);
    key_row.add_suffix(&key_drop);
    options_group.add(&key_row);

    let microsoft = adw::SwitchRow::new();
    microsoft.set_title("Include Microsoft keys");
    microsoft.set_subtitle("Keeps Windows and vendor option ROMs bootable");
    microsoft.set_active(true);
    options_group.add(&microsoft);

    let firmware_builtin = adw::SwitchRow::new();
    firmware_builtin.set_title("Include firmware built-in keys");
    firmware_builtin.set_subtitle("Framework and other vendors that need their own certificates");
    options_group.add(&firmware_builtin);

    content.append(&options_group);

    let log = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .css_classes(["caption"])
        .build();
    content.append(&log);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let refresh_btn = gtk::Button::with_label("Refresh");
    let apply_btn = gtk::Button::with_label("Write configuration");
    let reboot_btn = gtk::Button::with_label("Reboot now");
    let next_btn = gtk::Button::with_label("Next step");
    next_btn.add_css_class("suggested-action");
    buttons.append(&refresh_btn);
    buttons.append(&apply_btn);
    buttons.append(&reboot_btn);
    buttons.append(&next_btn);
    content.append(&buttons);

    content.append(&status.group);

    let state: Rc<RefCell<Option<SecureBootState>>> = Rc::new(RefCell::new(None));
    let configured = Rc::new(Cell::new(configured));
    let written = Rc::new(Cell::new(false));

    let options_for: Rc<dyn Fn() -> SetupOptions> = {
        let key_drop = key_drop.clone();
        let microsoft = microsoft.clone();
        let firmware_builtin = firmware_builtin.clone();
        Rc::new(move || SetupOptions {
            key_action: key_action(key_drop.selected()),
            include_microsoft: microsoft.is_active(),
            include_firmware_builtin: firmware_builtin.is_active(),
        })
    };

    let update_ui: Rc<dyn Fn()> = {
        let state = state.clone();
        let configured = configured.clone();
        let stage_title = stage_title.clone();
        let stage_detail = stage_detail.clone();
        let stage_rows = stage_rows.clone();
        let key_drop = key_drop.clone();
        let apply_btn = apply_btn.clone();
        let reboot_btn = reboot_btn.clone();
        let next_btn = next_btn.clone();
        let log = log.clone();
        let options_for = options_for.clone();
        Rc::new(move || {
            let Some(state) = state.borrow().clone() else {
                return;
            };
            if !state.keys_present && key_action(key_drop.selected()) == KeyAction::Keep {
                key_drop.set_selected(1);
            }
            if state.keys_present && key_action(key_drop.selected()) == KeyAction::Create {
                key_drop.set_selected(0);
            }

            let current = secureboot::stage(&state, configured.get());
            stage_title.set_text(current.title());
            stage_detail.set_text(current.detail());
            let position = stage_position(current);
            for (stage, icon, label) in &stage_rows {
                let index = stage_position(*stage);
                if *stage == current {
                    icon.set_icon_name(Some("go-next-symbolic"));
                    label.add_css_class("accent");
                } else {
                    label.remove_css_class("accent");
                    if index < position {
                        icon.set_icon_name(Some("emblem-ok-symbolic"));
                    } else {
                        icon.set_icon_name(Some("radio-symbolic"));
                    }
                }
            }

            let options = options_for();
            let label = match current {
                SetupStage::NotUefi => "UEFI required",
                SetupStage::Keys => options.key_action.label(),
                SetupStage::Config => "Write configuration",
                SetupStage::Rebuild => "Rebuild now",
                SetupStage::ClearKeys | SetupStage::EnableSecureBoot => "Reboot to firmware",
                SetupStage::Enroll => "Enroll keys",
                SetupStage::Verify => "Rebuild and re-check",
                SetupStage::Done => "Close",
            };
            next_btn.set_label(label);
            next_btn.set_visible(current != SetupStage::Done);
            next_btn.set_sensitive(current != SetupStage::NotUefi);
            apply_btn.set_visible(current != SetupStage::Config && current != SetupStage::Done);
            reboot_btn.set_visible(current == SetupStage::Rebuild);
            log.set_text(&format!(
                "{} · {}",
                badge(&state, configured.get()),
                stage_detail.text()
            ));
            if let Some(error) = options.validation_error(&state) {
                log.set_text(&error);
            }
        })
    };

    let refresh: Rc<dyn Fn()> = {
        let hook = ctx.secureboot_status.clone();
        let state = state.clone();
        let status = status.clone();
        let update_ui = update_ui.clone();
        let configured = configured.clone();
        let log = log.clone();
        Rc::new(move || {
            let Some(hook) = hook.clone() else {
                log.set_text("Secure Boot support is not available");
                return;
            };
            status.set_checking();
            let state = state.clone();
            let status = status.clone();
            let update_ui = update_ui.clone();
            let configured = configured.clone();
            let log = log.clone();
            hook(
                true,
                Box::new(move |result| {
                    status.update(&result, configured.get());
                    match result {
                        Ok(value) => {
                            *state.borrow_mut() = Some(value);
                        }
                        Err(error) => log.set_text(&error),
                    }
                    update_ui();
                }),
            );
        })
    };

    let write: Rc<dyn Fn()> = {
        let ctx = ctx.clone();
        let record = record.clone();
        let options_for = options_for.clone();
        let configured = configured.clone();
        let log = log.clone();
        let refresh = refresh.clone();
        let on_done = on_done.clone();
        let written = written.clone();
        let apply_btn = apply_btn.clone();
        Rc::new(move || {
            if written.get() {
                log.set_text("Configuration already written. Reopen the dialog to change it.");
                return;
            }
            let options = options_for();
            let Some(write) = ctx.secureboot_write.clone() else {
                log.set_text("Cannot write the configuration");
                return;
            };
            match write(record.clone(), options) {
                Ok(summary) => {
                    configured.set(true);
                    written.set(true);
                    apply_btn.set_sensitive(false);
                    log.set_text(&summary);
                    refresh();
                    on_done();
                }
                Err(error) => log.set_text(&error),
            }
        })
    };

    {
        let refresh = refresh.clone();
        refresh_btn.connect_clicked(move |_| refresh());
    }
    {
        let write = write.clone();
        apply_btn.connect_clicked(move |_| write());
    }
    {
        let ctx = ctx.clone();
        let log = log.clone();
        let on_done = on_done.clone();
        let dialog = dialog.clone();
        reboot_btn.connect_clicked(move |_| confirm_reboot(&dialog, &ctx, &log, &on_done));
    }
    {
        let ctx = ctx.clone();
        let state = state.clone();
        let configured = configured.clone();
        let options_for = options_for.clone();
        let log = log.clone();
        let refresh = refresh.clone();
        let write = write.clone();
        let on_done = on_done.clone();
        let dialog = dialog.clone();
        next_btn.connect_clicked(move |_| {
            let Some(current) = state.borrow().clone() else {
                return;
            };
            let options = options_for();
            if let Some(error) = options.validation_error(&current) {
                log.set_text(&error);
                return;
            }
            match secureboot::stage(&current, configured.get()) {
                SetupStage::Keys => {
                    let commands = secureboot::setup_commands(&options, &current);
                    let log = log.clone();
                    let refresh = refresh.clone();
                    let on_done = on_done.clone();
                    run_captured(&ctx, &commands, &log, move |result| match result {
                        Ok(text) => {
                            if !text.trim().is_empty() {
                                log.set_text(text.trim());
                            }
                            refresh();
                            on_done();
                        }
                        Err(error) => log.set_text(&error),
                    });
                }
                SetupStage::Config => write(),
                SetupStage::Rebuild => {
                    if let Some(rebuild) = &ctx.secureboot_rebuild {
                        rebuild();
                        log.set_text(
                            "Rebuild started in the terminal. Use Reboot now when it finishes.",
                        );
                        on_done();
                    } else {
                        log.set_text("No rebuild command is configured");
                    }
                }
                SetupStage::ClearKeys | SetupStage::EnableSecureBoot => {
                    let stage = secureboot::stage(&current, configured.get());
                    let log = log.clone();
                    let on_done = on_done.clone();
                    open_firmware_instructions(&dialog, &ctx, stage, &log, &on_done);
                }
                SetupStage::Enroll => {
                    if !current.in_setup_mode() {
                        log.set_text(
                            "The firmware is not in Setup Mode. Reboot into firmware and reset it before enrolling.",
                        );
                        return;
                    }
                    let commands = secureboot::setup_commands(&options, &current);
                    let log = log.clone();
                    let refresh = refresh.clone();
                    let on_done = on_done.clone();
                    run_captured(&ctx, &commands, &log, move |result| match result {
                        Ok(text) => {
                            let mut message = if text.trim().is_empty() {
                                "Keys enrolled.".to_string()
                            } else {
                                text.trim().to_string()
                            };
                            if let Err(error) = secureboot::write_enrollment_marker() {
                                message.push_str(&format!(
                                    "\nCannot record enrollment: {error}"
                                ));
                            }
                            log.set_text(&message);
                            refresh();
                            on_done();
                        }
                        Err(error) => log.set_text(&error),
                    });
                }
                SetupStage::Verify => {
                    if let Some(rebuild) = &ctx.secureboot_rebuild {
                        rebuild();
                        log.set_text("Rebuild started in the terminal. Refresh when it finishes.");
                        on_done();
                    } else {
                        refresh();
                    }
                }
                SetupStage::Done => {
                    dialog.response(gtk::ResponseType::Close);
                }
                SetupStage::NotUefi => {}
            }
        });
    }

    dialog.connect_response(|dialog, _| dialog.close());
    refresh();
    dialog.present();
}

fn confirm_reboot(
    parent: &gtk::Dialog,
    ctx: &BuildContext,
    log: &gtk::Label,
    on_done: &Rc<dyn Fn()>,
) {
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading("Reboot to activate the new generation")
        .body(
            "The rebuild uses nixos-rebuild boot, so the Lanzaboote generation is only \
             active after a reboot.",
        )
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("reboot", "Reboot");
    dialog.set_response_appearance("reboot", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    let log = log.clone();
    let on_done = on_done.clone();
    dialog.connect_response(None, move |dialog, response| {
        if response == "reboot" {
            let commands = vec!["systemctl reboot".to_string()];
            run_commands(&ctx, &commands, &log, &on_done);
        }
        dialog.close();
    });
    dialog.present();
}

pub(crate) fn open_repair(
    parent: Option<&gtk::Window>,
    state: SecureBootState,
    ctx: &BuildContext,
    on_done: Rc<dyn Fn()>,
) {
    if !state.efi {
        show_message(
            parent,
            "UEFI required",
            "Secure Boot repair needs a UEFI boot.",
        );
        return;
    }
    if !state.keys_present {
        show_message(
            parent,
            "No key bundle",
            "There is no key bundle in /var/lib/sbctl. Run Set up Secure Boot first.",
        );
        return;
    }
    if state.secure_boot_enabled() {
        show_message(
            parent,
            "Secure Boot is already active",
            "Disable Secure Boot in firmware before repairing the keys.",
        );
        return;
    }

    let parent_owned = parent.map(|window| window.clone());
    if state.in_setup_mode() {
        let dialog = adw::MessageDialog::builder()
            .modal(true)
            .heading("Re-enroll Secure Boot keys")
            .body(
                "The firmware is in Setup Mode. Re-enroll the current /var/lib/sbctl bundle, \
                 including the Microsoft certificates for Windows. Confirm the BitLocker recovery \
                 key in Windows first.",
            )
            .build();
        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("repair", "Re-enroll keys");
        dialog.set_response_appearance("repair", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let ctx = ctx.clone();
        let on_done = on_done.clone();
        let parent = parent_owned.clone();
        dialog.connect_response(None, move |dialog, response| {
            if response == "repair" {
                let commands = vec!["sbctl enroll-keys --microsoft".to_string()];
                let log = gtk::Label::new(None);
                let on_done = on_done.clone();
                let parent = parent.clone();
                run_captured(&ctx, &commands, &log, move |result| match result {
                    Ok(_) => {
                        if let Err(error) = secureboot::write_enrollment_marker() {
                            show_message(
                                parent.as_ref(),
                                "Keys enrolled, enrollment marker failed",
                                &error,
                            );
                        } else {
                            show_message(
                                parent.as_ref(),
                                "Secure Boot keys re-enrolled",
                                "Reboot, enable Secure Boot in firmware, and select the Linux \
                                 Boot Manager. Do not reset to Setup Mode.",
                            );
                        }
                        on_done();
                    }
                    Err(error) => {
                        show_message(parent.as_ref(), "Re-enrollment failed", &error);
                    }
                });
            }
            dialog.close();
        });
        dialog.present();
        return;
    }

    let dialog = adw::MessageDialog::builder()
        .modal(true)
        .heading("Firmware is not in Setup Mode")
        .body(
            "The firmware still holds a different key bundle. Enter firmware settings, open \
             Security > Secure Boot > Key Management, and select Reset to Setup Mode or delete \
             the Platform Key. Do not select Clear All Secure Boot Keys if a reset option exists; \
             it can drop the Forbidden Signature Database. Save and exit, boot back into NixOS, \
             then run Repair Secure Boot keys again.",
        )
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("reboot", "Reboot to firmware");
    dialog.set_response_appearance("reboot", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    let parent = parent_owned;
    dialog.connect_response(None, move |dialog, response| {
        if response == "reboot" {
            let commands = vec!["systemctl reboot --firmware-setup".to_string()];
            let log = gtk::Label::new(None);
            let parent = parent.clone();
            run_captured(&ctx, &commands, &log, move |result| {
                if let Err(error) = result {
                    show_message(parent.as_ref(), "Cannot reboot to firmware", &error);
                }
            });
        }
        dialog.close();
    });
    dialog.present();
}

fn show_message(parent: Option<&gtk::Window>, heading: &str, body: &str) {
    let dialog = adw::MessageDialog::builder()
        .modal(true)
        .heading(heading)
        .body(body)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }
    dialog.add_response("close", "Close");
    dialog.set_close_response("close");
    dialog.connect_response(None, |dialog, _| dialog.close());
    dialog.present();
}

fn run_captured(
    ctx: &BuildContext,
    commands: &[String],
    log: &gtk::Label,
    done: impl FnOnce(Result<String, String>) + 'static,
) {
    if commands.is_empty() {
        log.set_text("There is no privileged action left for this step");
        return;
    }
    let Some(exec) = ctx.secureboot_exec.clone() else {
        log.set_text("Cannot run privileged commands");
        return;
    };
    exec(commands.to_vec(), Box::new(done));
}

fn open_firmware_instructions(
    parent: &gtk::Dialog,
    ctx: &BuildContext,
    stage: SetupStage,
    log: &gtk::Label,
    on_done: &Rc<dyn Fn()>,
) {
    let (heading, body) = match stage {
        SetupStage::ClearKeys => (
            "Clear Secure Boot keys in firmware",
            "The firmware still holds a different key bundle. Enter firmware settings, open \
             Security > Secure Boot > Key Management, and select Reset to Setup Mode or delete \
             the Platform Key. Do not select Clear All Secure Boot Keys if a reset option exists; \
             it can drop the Forbidden Signature Database. Save and exit, then boot back into \
             NixOS.",
        ),
        SetupStage::EnableSecureBoot => (
            "Enable Secure Boot in firmware",
            "The keys are enrolled. Enter firmware settings, open Security > Secure Boot, and \
             enable Secure Boot. Do not reset to Setup Mode; that clears the enrolled keys. Save \
             and exit.",
        ),
        _ => return,
    };
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading(heading)
        .body(body)
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("reboot", "Reboot to firmware");
    dialog.set_response_appearance("reboot", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    let log = log.clone();
    let on_done = on_done.clone();
    dialog.connect_response(None, move |dialog, response| {
        if response == "reboot" {
            let commands = vec!["systemctl reboot --firmware-setup".to_string()];
            let log = log.clone();
            let on_done = on_done.clone();
            run_captured(&ctx, &commands, &log, move |result| {
                if let Err(error) = result {
                    log.set_text(&error);
                }
                on_done();
            });
        }
        dialog.close();
    });
    dialog.present();
}

fn run_commands(ctx: &BuildContext, commands: &[String], log: &gtk::Label, on_done: &Rc<dyn Fn()>) {
    if commands.is_empty() {
        log.set_text("There is no privileged action left for this step");
        return;
    }
    let Some(run) = ctx.secureboot_run.clone() else {
        log.set_text("Cannot run privileged commands");
        return;
    };
    match run(commands) {
        Ok(()) => {
            log.set_text(
                "Command started in the terminal. Refresh when it finishes, or reboot if the flow asks for it.",
            );
            on_done();
        }
        Err(error) => log.set_text(&error),
    }
}
