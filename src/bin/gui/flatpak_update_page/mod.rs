mod process;

use crate::apt_package_row::AptPackageRow;
use crate::flatpak_ref_row::FlatpakRefRow;
use adw::gio::SimpleAction;
use adw::prelude::*;
use gtk::glib::*;
use gtk::*;
use libflatpak::prelude::*;
use libflatpak::InstalledRef;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::thread;

#[derive(Clone)]
pub struct FlatpakRefStruct {
    pub ref_name: String,
    pub name: String,
    pub arch: String,
    pub summary: String,
    pub remote_name: String,
    pub installed_size_installed: u64,
    pub installed_size_remote: u64,
    pub download_size: u64,
    pub ref_format: String,
    pub is_system: bool,
    pub is_last: bool,
}
pub fn flatpak_update_page(
    window: adw::ApplicationWindow,
    retry_signal_action: &SimpleAction,
) -> gtk::Box {
    let (appstream_sync_percent_sender, appstream_sync_percent_receiver) =
        async_channel::unbounded::<u32>();
    let appstream_sync_percent_sender = appstream_sync_percent_sender.clone();
    let (appstream_sync_status_sender, appstream_sync_status_receiver) =
        async_channel::unbounded::<String>();
    let appstream_sync_status_sender = appstream_sync_status_sender.clone();
    let appstream_sync_status_sender_clone0 = appstream_sync_status_sender.clone();

    let system_refs_for_upgrade_vec: Rc<RefCell<Vec<FlatpakRefRow>>> =
        Rc::new(RefCell::new(Vec::new()));

    let user_refs_for_upgrade_vec: Rc<RefCell<Vec<FlatpakRefRow>>> =
        Rc::new(RefCell::new(Vec::new()));

    let system_refs_for_upgrade_vec_all: Rc<RefCell<Vec<FlatpakRefRow>>> =
        Rc::new(RefCell::new(Vec::new()));

    let user_refs_for_upgrade_vec_all: Rc<RefCell<Vec<FlatpakRefRow>>> =
        Rc::new(RefCell::new(Vec::new()));

    let cancellable_no = libflatpak::gio::Cancellable::NONE;

    thread::spawn(move || {
        let cancellable_no = libflatpak::gio::Cancellable::NONE;
        let flatpak_system_installation =
            libflatpak::Installation::new_user(cancellable_no).unwrap();
        if let Ok(remotes) =
            libflatpak::Installation::list_remotes(&flatpak_system_installation, cancellable_no)
        {
            for remote in remotes {
                if remote.is_disabled() {
                    continue;
                };
                let mut remote_clousre = |status: &str, progress: u32, _: bool| {
                    appstream_sync_percent_sender
                        .send_blocking(progress)
                        .expect("appstream_sync_percent_receiver closed");
                    appstream_sync_status_sender
                        .send_blocking(format!(
                            "{} - {}: {}",
                            t!("flatpak_type_system"),
                            remote.name().unwrap_or("Unknown remote".into()),
                            status
                        ))
                        .expect("appstream_sync_status_receiver closed");
                };
                match libflatpak::Installation::update_appstream_full_sync(
                    &flatpak_system_installation,
                    &remote.name().unwrap(),
                    None,
                    Some(&mut remote_clousre),
                    cancellable_no,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        appstream_sync_status_sender
                            .send_blocking(e.to_string())
                            .expect("appstream_sync_status_receiver closed");
                        appstream_sync_status_sender
                            .send_blocking("FN_OVERRIDE_FAILED".to_owned())
                            .expect("appstream_sync_status_receiver closed");
                        break;
                    }
                }
            }
        }
        let flatpak_user_installation = libflatpak::Installation::new_user(cancellable_no).unwrap();
        if let Ok(remotes) =
            libflatpak::Installation::list_remotes(&flatpak_user_installation, cancellable_no)
        {
            for remote in remotes {
                if remote.is_disabled() {
                    continue;
                };
                let mut remote_clousre = |status: &str, progress: u32, _: bool| {
                    appstream_sync_percent_sender
                        .send_blocking(progress)
                        .expect("appstream_sync_percent_receiver closed");
                    appstream_sync_status_sender
                        .send_blocking(format!(
                            "{} - {}: {}",
                            t!("flatpak_type_user"),
                            remote.name().unwrap_or("Unknown remote".into()),
                            status
                        ))
                        .expect("appstream_sync_status_receiver closed");
                };
                match libflatpak::Installation::update_appstream_full_sync(
                    &flatpak_user_installation,
                    &remote.name().unwrap(),
                    None,
                    Some(&mut remote_clousre),
                    cancellable_no,
                ) {
                    Ok(_) => {
                        appstream_sync_status_sender
                            .send_blocking("FN_OVERRIDE_SUCCESSFUL".to_owned())
                            .expect("appstream_sync_status_receiver closed");
                    }
                    Err(e) => {
                        appstream_sync_status_sender
                            .send_blocking(e.to_string())
                            .expect("appstream_sync_status_receiver closed");
                        appstream_sync_status_sender
                            .send_blocking("FN_OVERRIDE_FAILED".to_owned())
                            .expect("appstream_sync_status_receiver closed");
                        break;
                    }
                }
            }
        }
    });

    let main_box = Box::builder()
        .hexpand(true)
        .vexpand(true)
        .orientation(Orientation::Vertical)
        .build();

    let searchbar = SearchEntry::builder()
        .search_delay(500)
        .margin_top(15)
        .margin_bottom(15)
        .margin_end(30)
        .margin_start(30)
        .build();
    searchbar.add_css_class("rounded-all-25");

    let packages_boxedlist = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .sensitive(false)
        .build();
    packages_boxedlist.add_css_class("boxed-list");
    packages_boxedlist.add_css_class("round-all-scroll");

    let packages_viewport = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .has_frame(true)
        .margin_bottom(15)
        .margin_top(15)
        .margin_end(15)
        .margin_start(15)
        .height_request(390)
        .child(&packages_boxedlist)
        .build();
    packages_viewport.add_css_class("round-all-scroll");

    let packages_no_viewport_page = adw::StatusPage::builder()
        .icon_name("emblem-default-symbolic")
        .title(t!("flatpak_packages_no_viewport_page_title"))
        .hexpand(true)
        .vexpand(true)
        .build();

    let viewport_bin = adw::Bin::builder()
        .child(&packages_no_viewport_page)
        .build();

    let flatpak_update_dialog_child_box = Box::builder().orientation(Orientation::Vertical).build();

    let flatpak_update_dialog_progress_bar =
        ProgressBar::builder().show_text(true).hexpand(true).build();

    let flatpak_update_dialog_spinner = Spinner::builder()
        .hexpand(true)
        .valign(Align::Start)
        .halign(Align::Center)
        .spinning(true)
        .height_request(128)
        .width_request(128)
        .build();

    flatpak_update_dialog_child_box.append(&flatpak_update_dialog_spinner);
    flatpak_update_dialog_child_box.append(&flatpak_update_dialog_progress_bar);

    let flatpak_update_dialog = adw::MessageDialog::builder()
        .transient_for(&window)
        .extra_child(&flatpak_update_dialog_child_box)
        .heading(t!("flatpak_update_dialog_heading"))
        .width_request(500)
        .build();

    flatpak_update_dialog.add_response(
        "flatpak_update_dialog_retry",
        &t!("flatpak_update_dialog_retry_label").to_string(),
    );

    flatpak_update_dialog.set_response_appearance(
        "flatpak_update_dialog_retry",
        adw::ResponseAppearance::Suggested,
    );

    flatpak_update_dialog.set_response_enabled("flatpak_update_dialog_retry", false);

    let retry_signal_action0 = retry_signal_action.clone();

    flatpak_update_dialog
        .clone()
        .choose(None::<&gio::Cancellable>, move |choice| {
            if choice == "flatpak_update_dialog_retry" {
                retry_signal_action0.activate(None);
            }
        });

    let bottom_bar = Box::builder().valign(Align::End).build();

    let select_button = Button::builder()
        .halign(Align::End)
        .valign(Align::Center)
        .hexpand(true)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(15)
        .label(t!("select_button_deselect_all"))
        .build();

    select_button.connect_clicked(clone!(
        #[weak]
        select_button,
        #[weak]
        packages_boxedlist,
        move |_| {
            let select_button_label = select_button.label().unwrap();
            let value_to_mark = if select_button_label == t!("select_button_select_all").to_string()
            {
                true
            } else if select_button_label == t!("select_button_deselect_all").to_string() {
                false
            } else {
                panic!("Unexpected label on selection button")
            };
            set_all_flatpak_row_marks_to(&packages_boxedlist, value_to_mark)
        }
    ));

    let update_button = Button::builder()
        .halign(Align::End)
        .valign(Align::Center)
        .hexpand(false)
        .margin_start(10)
        .margin_end(30)
        .margin_bottom(15)
        .label(t!("update_button_label"))
        .build();
    update_button.add_css_class("destructive-action");

    let system_refs_for_upgrade_vec_all_clone0 = &system_refs_for_upgrade_vec_all.clone();
    let user_refs_for_upgrade_vec_all_clone0 = user_refs_for_upgrade_vec_all.clone();

    let system_refs_for_upgrade_vec_clone0 = system_refs_for_upgrade_vec.clone();
    let user_refs_for_upgrade_vec_clone0 = user_refs_for_upgrade_vec.clone();

    update_button.connect_clicked(clone!(
        #[weak]
        window,
        #[weak]
        retry_signal_action,
        #[strong]
        system_refs_for_upgrade_vec_all_clone0,
        #[strong]
        user_refs_for_upgrade_vec_all_clone0,
        move |_| {
            process::flatpak_process_update(
                Some(&system_refs_for_upgrade_vec_clone0.borrow()),
                Some(&user_refs_for_upgrade_vec_clone0.borrow()),
                &system_refs_for_upgrade_vec_all_clone0.borrow(),
                &user_refs_for_upgrade_vec_all_clone0.borrow(),
                window,
                &retry_signal_action,
            )
        }
    ));

    bottom_bar.append(&select_button);
    bottom_bar.append(&update_button);

    let appstream_sync_percent_server_context = MainContext::default();
    // The main loop executes the asynchronous block
    appstream_sync_percent_server_context.spawn_local(clone!(
        #[weak]
        flatpak_update_dialog_progress_bar,
        async move {
            while let Ok(state) = appstream_sync_percent_receiver.recv().await {
                flatpak_update_dialog_progress_bar.set_fraction(state as f64 / 100.0)
            }
        }
    ));

    let appstream_sync_status_server_context = MainContext::default();
    // The main loop executes the asynchronous block
    appstream_sync_status_server_context.spawn_local(clone!(
        #[weak]
        flatpak_update_dialog,
        #[weak]
        flatpak_update_dialog_child_box,
        #[strong]
        packages_boxedlist,
        #[strong]
        system_refs_for_upgrade_vec_all,
        #[strong]
        user_refs_for_upgrade_vec_all,
        #[strong]
        system_refs_for_upgrade_vec,
        #[strong]
        user_refs_for_upgrade_vec,
        #[strong]
        viewport_bin,
        #[strong]
        packages_viewport,
        async move {
            while let Ok(state) = appstream_sync_status_receiver.recv().await {
                match state.as_ref() {
                    "FN_OVERRIDE_SUCCESSFUL" => {
                        let flatpak_system_installation =
                            libflatpak::Installation::new_system(cancellable_no).unwrap();
                        let flatpak_system_updates = flatpak_system_installation
                            .list_installed_refs_for_update(cancellable_no)
                            .unwrap();
                        let flatpak_system_transaction = libflatpak::Transaction::for_installation(
                            &flatpak_system_installation,
                            cancellable_no,
                        )
                        .unwrap();
                        //
                        let flatpak_user_installation =
                            libflatpak::Installation::new_user(cancellable_no).unwrap();
                        let flatpak_user_updates = flatpak_user_installation
                            .list_installed_refs_for_update(cancellable_no)
                            .unwrap();
                        let flatpak_user_transaction = libflatpak::Transaction::for_installation(
                            &flatpak_user_installation,
                            cancellable_no,
                        )
                        .unwrap();
                        //
                        let mut system_last_triggered = false;
                        let mut user_last_triggered = false;
                        //
                        if !flatpak_system_updates.is_empty() || !flatpak_user_updates.is_empty() {
                          viewport_bin.set_child(Some(&packages_viewport));  
                        }
                        //
                        if !flatpak_system_updates.is_empty() {
                            let mut flatpak_system_updates_iter =
                                &mut flatpak_system_updates.iter().peekable();
                            //
                            while let Some(flatpak_ref) = flatpak_system_updates_iter.next() {
                                let mut remote_flatpak_ref: Option<libflatpak::RemoteRef> = None;
                                while let Ok(remotes) = libflatpak::Installation::list_remotes(
                                    &flatpak_system_installation,
                                    cancellable_no,
                                ) {
                                    for remote in remotes {
                                        if remote.is_disabled() {
                                            continue;
                                        };
                                        match libflatpak::Installation::fetch_remote_ref_sync(
                                            &flatpak_system_installation,
                                            &match remote.name() {
                                                Some(t) => t,
                                                None => continue,
                                            },
                                            flatpak_ref.kind(),
                                            &match flatpak_ref.name() {
                                                Some(t) => t,
                                                None => continue,
                                            },
                                            flatpak_ref.arch().as_deref(),
                                            flatpak_ref.branch().as_deref(),
                                            cancellable_no,
                                        ) {
                                            Ok(t) => {
                                                remote_flatpak_ref = Some(t);
                                                break;
                                            }
                                            Err(_) => continue,
                                        }
                                    }
                                    if remote_flatpak_ref.is_some() {
                                        break;
                                    }
                                }
                                let flatref_struct = FlatpakRefStruct {
                                    ref_name: flatpak_ref
                                        .name()
                                        .unwrap_or("Unknown".into())
                                        .to_string(),
                                    name: flatpak_ref
                                        .appdata_name()
                                        .unwrap_or(flatpak_ref.name().unwrap_or("Unknown".into()))
                                        .to_string(),
                                    arch: flatpak_ref
                                        .arch()
                                        .unwrap_or("Unknown Arch".into())
                                        .to_string(),
                                    summary: flatpak_ref
                                        .appdata_summary()
                                        .unwrap_or("No Summary".into())
                                        .to_string(),
                                    remote_name: match remote_flatpak_ref {
                                        Some(ref t) => {
                                            t.remote_name().unwrap_or("Unknown".into()).to_string()
                                        }
                                        None => "Unknown".into(),
                                    },
                                    installed_size_installed: flatpak_ref.installed_size(),
                                    installed_size_remote: match remote_flatpak_ref {
                                        Some(ref t) => t.installed_size(),
                                        None => 0,
                                    },
                                    download_size: match remote_flatpak_ref {
                                        Some(t) => t.download_size(),
                                        None => 0,
                                    },
                                    ref_format: flatpak_ref.format_ref().unwrap().into(),
                                    is_system: true,
                                    is_last: flatpak_system_updates_iter.peek().is_none(),
                                };

                                let flatpak_row = FlatpakRefRow::new(&flatref_struct);

                                system_refs_for_upgrade_vec
                                    .borrow_mut()
                                    .push(flatpak_row.clone());

                                system_refs_for_upgrade_vec_all
                                    .borrow_mut()
                                    .push(flatpak_row.clone());

                                flatpak_row.connect_closure(
                                    "checkbutton-toggled",
                                    false,
                                    closure_local!(
                                        #[strong]
                                        select_button,
                                        #[strong]
                                        update_button,
                                        #[strong]
                                        packages_boxedlist,
                                        #[strong]
                                        system_refs_for_upgrade_vec,
                                        move |flatpak_row: FlatpakRefRow| {
                                            if is_widget_select_all_ready(&packages_boxedlist) {
                                                select_button.set_label(
                                                    &t!("select_button_select_all").to_string(),
                                                );
                                            } else {
                                                select_button.set_label(
                                                    &t!("select_button_deselect_all").to_string(),
                                                );
                                            }
                                            update_button.set_sensitive(!is_all_children_unmarked(
                                                &packages_boxedlist,
                                            ));
                                            system_refs_for_upgrade_vec
                                                .borrow_mut()
                                                .push(flatpak_row);
                                        }
                                    ),
                                );
                                flatpak_row.connect_closure(
                                    "checkbutton-untoggled",
                                    false,
                                    closure_local!(
                                        #[strong]
                                        select_button,
                                        #[strong]
                                        update_button,
                                        #[strong]
                                        packages_boxedlist,
                                        #[strong]
                                        system_refs_for_upgrade_vec,
                                        move |flatpak_row: FlatpakRefRow| {
                                            select_button.set_label(
                                                &t!("select_button_select_all").to_string(),
                                            );
                                            update_button.set_sensitive(!is_all_children_unmarked(
                                                &packages_boxedlist,
                                            ));
                                            system_refs_for_upgrade_vec.borrow_mut().retain(|x| {
                                                x.flatref_ref_format()
                                                    != flatpak_row.flatref_ref_format()
                                            });
                                        }
                                    ),
                                );

                                packages_boxedlist.append(&flatpak_row);
                                if flatref_struct.is_system && flatref_struct.is_last
                                {
                                    system_last_triggered = true
                                }
                            }
                        } else {
                            system_last_triggered = true
                        }
                        if !flatpak_user_updates.is_empty() {
                            let mut flatpak_user_updates_iter =
                                &mut flatpak_user_updates.iter().peekable();
                            //
                            while let Some(flatpak_ref) = flatpak_user_updates_iter.next() {
                                let mut remote_flatpak_ref: Option<libflatpak::RemoteRef> = None;
                                while let Ok(remotes) = libflatpak::Installation::list_remotes(
                                    &flatpak_user_installation,
                                    cancellable_no,
                                ) {
                                    for remote in remotes {
                                        if remote.is_disabled() {
                                            continue;
                                        };
                                        match libflatpak::Installation::fetch_remote_ref_sync(
                                            &flatpak_user_installation,
                                            &match remote.name() {
                                                Some(t) => t,
                                                None => continue,
                                            },
                                            flatpak_ref.kind(),
                                            &match flatpak_ref.name() {
                                                Some(t) => t,
                                                None => continue,
                                            },
                                            flatpak_ref.arch().as_deref(),
                                            flatpak_ref.branch().as_deref(),
                                            cancellable_no,
                                        ) {
                                            Ok(t) => {
                                                remote_flatpak_ref = Some(t);
                                                break;
                                            }
                                            Err(_) => continue,
                                        }
                                    }
                                    if remote_flatpak_ref.is_some() {
                                        break;
                                    }
                                }
                                let flatref_struct = FlatpakRefStruct {
                                    ref_name: flatpak_ref
                                        .name()
                                        .unwrap_or("Unknown".into())
                                        .to_string(),
                                    name: flatpak_ref
                                        .appdata_name()
                                        .unwrap_or(flatpak_ref.name().unwrap_or("Unknown".into()))
                                        .to_string(),
                                    arch: flatpak_ref
                                        .arch()
                                        .unwrap_or("Unknown Arch".into())
                                        .to_string(),
                                    summary: flatpak_ref
                                        .appdata_summary()
                                        .unwrap_or("No Summary".into())
                                        .to_string(),
                                    remote_name: match remote_flatpak_ref {
                                        Some(ref t) => {
                                            t.remote_name().unwrap_or("Unknown".into()).to_string()
                                        }
                                        None => "Unknown".into(),
                                    },
                                    installed_size_installed: flatpak_ref.installed_size(),
                                    installed_size_remote: match remote_flatpak_ref {
                                        Some(ref t) => t.installed_size(),
                                        None => 0,
                                    },
                                    download_size: match remote_flatpak_ref {
                                        Some(t) => t.download_size(),
                                        None => 0,
                                    },
                                    ref_format: flatpak_ref.format_ref().unwrap().into(),
                                    is_system: false,
                                    is_last: flatpak_user_updates_iter.peek().is_none(),
                                };

                                let flatpak_row = FlatpakRefRow::new(&flatref_struct);

                                user_refs_for_upgrade_vec
                                    .borrow_mut()
                                    .push(flatpak_row.clone());

                                user_refs_for_upgrade_vec_all
                                    .borrow_mut()
                                    .push(flatpak_row.clone());

                                flatpak_row.connect_closure(
                                    "checkbutton-toggled",
                                    false,
                                    closure_local!(
                                        #[strong]
                                        select_button,
                                        #[strong]
                                        update_button,
                                        #[strong]
                                        packages_boxedlist,
                                        #[strong]
                                        user_refs_for_upgrade_vec,
                                        move |flatpak_row: FlatpakRefRow| {
                                            if is_widget_select_all_ready(&packages_boxedlist) {
                                                select_button.set_label(
                                                    &t!("select_button_select_all").to_string(),
                                                );
                                            } else {
                                                select_button.set_label(
                                                    &t!("select_button_deselect_all").to_string(),
                                                );
                                            }
                                            update_button.set_sensitive(!is_all_children_unmarked(
                                                &packages_boxedlist,
                                            ));
                                            user_refs_for_upgrade_vec
                                                .borrow_mut()
                                                .push(flatpak_row);
                                        }
                                    ),
                                );
                                flatpak_row.connect_closure(
                                    "checkbutton-untoggled",
                                    false,
                                    closure_local!(
                                        #[strong]
                                        select_button,
                                        #[strong]
                                        update_button,
                                        #[strong]
                                        packages_boxedlist,
                                        #[strong]
                                        user_refs_for_upgrade_vec,
                                        move |flatpak_row: FlatpakRefRow| {
                                            select_button.set_label(
                                                &t!("select_button_select_all").to_string(),
                                            );
                                            update_button.set_sensitive(!is_all_children_unmarked(
                                                &packages_boxedlist,
                                            ));
                                            user_refs_for_upgrade_vec.borrow_mut().retain(|x| {
                                                x.flatref_ref_format()
                                                    != flatpak_row.flatref_ref_format()
                                            });
                                        }
                                    ),
                                );
                                packages_boxedlist.append(&flatpak_row);
                                if !flatref_struct.is_system && flatref_struct.is_last
                                {
                                    user_last_triggered = true
                                }
                            }
                        } else {
                            user_last_triggered = true
                        }
                        if user_last_triggered && system_last_triggered {
                            packages_boxedlist.set_sensitive(true);
                        }
                        flatpak_update_dialog.close();
                    }
                    "FN_OVERRIDE_FAILED" => {
                        flatpak_update_dialog_child_box.set_visible(false);
                        flatpak_update_dialog.set_extra_child(Some(
                            &Image::builder()
                                .pixel_size(128)
                                .icon_name("dialog-error-symbolic")
                                .halign(Align::Center)
                                .build(),
                        ));
                        flatpak_update_dialog.set_title(Some(
                            &t!("flatpak_update_dialog_status_failed").to_string(),
                        ));
                        flatpak_update_dialog
                            .set_response_enabled("flatpak_update_dialog_retry", true);
                    }
                    _ => flatpak_update_dialog.set_body(&state),
                }
            }
        }
    ));

    searchbar.connect_search_changed(clone!(
        #[weak]
        searchbar,
        #[weak]
        packages_boxedlist,
        move |_| {
            let mut counter = packages_boxedlist.first_child();
            while let Some(row) = counter {
                if row.widget_name() == "FlatpakRefRow" {
                    if !searchbar.text().is_empty() {
                        if row
                            .property::<String>("flatref-name")
                            .to_lowercase()
                            .contains(&searchbar.text().to_string().to_lowercase())
                            || row
                                .property::<String>("flatref-ref-name")
                                .to_lowercase()
                                .contains(&searchbar.text().to_string().to_lowercase())
                        {
                            row.set_property("visible", true);
                            searchbar.grab_focus();
                        } else {
                            row.set_property("visible", false);
                        }
                    } else {
                        row.set_property("visible", true);
                    }
                }
                counter = row.next_sibling();
            }
        }
    ));

    main_box.append(&searchbar);
    main_box.append(&viewport_bin);
    main_box.append(&bottom_bar);

    flatpak_update_dialog.present();
    main_box
}

fn is_widget_select_all_ready(parent_listbox: &impl adw::prelude::IsA<ListBox>) -> bool {
    let mut is_ready = false;
    let mut child_counter = parent_listbox.borrow().first_child();
    while let Some(child) = child_counter {
        let next_child = child.next_sibling();
        let downcast = child.downcast::<FlatpakRefRow>().unwrap();
        if !downcast.flatref_marked() {
            is_ready = true;
            break;
        }
        child_counter = next_child
    }
    is_ready
}

fn is_all_children_unmarked(parent_listbox: &impl adw::prelude::IsA<ListBox>) -> bool {
    let mut is_all_unmarked = true;
    let mut child_counter = parent_listbox.borrow().first_child();
    while let Some(child) = child_counter {
        let next_child = child.next_sibling();
        let downcast = child.downcast::<FlatpakRefRow>().unwrap();
        if downcast.flatref_marked() {
            is_all_unmarked = false;
            break;
        }
        child_counter = next_child
    }
    is_all_unmarked
}

fn set_all_flatpak_row_marks_to(parent_listbox: &impl adw::prelude::IsA<ListBox>, value: bool) {
    let mut child_counter = parent_listbox.borrow().first_child();
    while let Some(child) = child_counter {
        let next_child = child.next_sibling();
        let downcast = child.downcast::<FlatpakRefRow>().unwrap();
        downcast.set_flatref_marked(value);
        child_counter = next_child
    }
}
