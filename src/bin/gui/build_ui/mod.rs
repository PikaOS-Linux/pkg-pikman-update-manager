use crate::apt_update_page;
use crate::config::{APP_GITHUB, APP_ICON, APP_ID, VERSION};
use adw::prelude::*;
use adw::*;
use gtk::glib::{clone, MainContext};
use gtk::License;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::thread;

pub fn build_ui(app: &Application) {
    // setup glib
    glib::set_prgname(Some(t!("application_name").to_string()));
    glib::set_application_name(&t!("application_name").to_string());

    let internet_connected = Rc::new(RefCell::new(false));
    let (internet_loop_sender, internet_loop_receiver) = async_channel::unbounded();
    let internet_loop_sender = internet_loop_sender.clone();

    thread::spawn(move || loop {
        match Command::new("ping").arg("google.com").arg("-c 1").output() {
            Ok(t) if t.status.success() => internet_loop_sender
                .send_blocking(true)
                .expect("The channel needs to be open"),
            _ => internet_loop_sender
                .send_blocking(false)
                .expect("The channel needs to be open"),
        };
        thread::sleep(std::time::Duration::from_secs(5));
    });

    let window_banner = Banner::builder().revealed(false).build();

    let internet_connected_status = internet_connected.clone();

    let internet_loop_context = MainContext::default();
    // The main loop executes the asynchronous block
    internet_loop_context.spawn_local(clone!(
        #[weak]
        window_banner,
        async move {
            while let Ok(state) = internet_loop_receiver.recv().await {
                let banner_text = t!("banner_text_no_internet").to_string();
                if state == true {
                    *internet_connected_status.borrow_mut() = true;
                    if window_banner.title() == banner_text {
                        window_banner.set_revealed(false)
                    }
                } else {
                    *internet_connected_status.borrow_mut() = false;
                    window_banner.set_title(&banner_text);
                    window_banner.set_revealed(true)
                }
            }
        }
    ));

    let window_headerbar = HeaderBar::builder()
        .title_widget(&WindowTitle::builder().title(t!("application_name")).build())
        .build();

    let window_adw_view_stack = ViewStack::builder()
        .hhomogeneous(true)
        .vhomogeneous(true)
        .build();

    let window_toolbar = ToolbarView::builder()
        .content(&window_adw_view_stack)
        .top_bar_style(ToolbarStyle::Flat)
        .bottom_bar_style(ToolbarStyle::Flat)
        .build();

    let window_adw_view_switcher_bar = ViewSwitcherBar::builder()
        .stack(&window_adw_view_stack)
        .reveal(true)
        .build();

    window_headerbar.pack_start(&window_adw_view_switcher_bar);

    window_toolbar.add_top_bar(&window_headerbar);
    window_toolbar.add_top_bar(&window_banner);

    // create the main Application window
    let window = ApplicationWindow::builder()
        // The text on the titlebar
        .title(t!("application_name"))
        // link it to the application "app"
        .application(app)
        // Add the box called "window_box" to it
        // Application icon
        .icon_name(APP_ICON)
        // Minimum Size/Default
        .width_request(700)
        .height_request(500)
        .content(&window_toolbar)
        // Startup
        .startup_id(APP_ID)
        // build the window
        .build();

    let credits_button = gtk::Button::builder()
        .icon_name("dialog-information-symbolic")
        .build();

    let refresh_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(t!("refresh_button_tooltip_text"))
        .build();

    let credits_window = AboutWindow::builder()
        .application_icon(APP_ICON)
        .application_name(t!("application_name"))
        .transient_for(&window)
        .version(VERSION)
        .hide_on_close(true)
        .developer_name(t!("developer_name"))
        .license_type(License::Mpl20)
        .issue_url(APP_GITHUB.to_owned() + "/issues")
        .build();

    window_headerbar.pack_end(&refresh_button);
    window_headerbar.pack_end(&credits_button);
    credits_button.connect_clicked(move |_| credits_window.present());

    // show the window

    window.present();

    // Apt Update Page
    let apt_retry_signal_action = gio::SimpleAction::new("retry", None);

    let apt_update_view_stack_bin = Bin::builder()
        .child(&apt_update_page::apt_update_page(
            window.clone(),
            &apt_retry_signal_action,
        ))
        .build();

    apt_retry_signal_action.connect_activate(clone!(
        #[weak]
        window,
        #[strong]
        apt_retry_signal_action,
        #[strong]
        apt_update_view_stack_bin,
        move |_, _| {
            apt_update_view_stack_bin.set_child(Some(&apt_update_page::apt_update_page(
                window,
                &apt_retry_signal_action,
            )));
        }
    ));

    window_adw_view_stack.add_titled_with_icon(
        &apt_update_view_stack_bin,
        Some("apt_update_page"),
        &t!("apt_update_page_title"),
        "software-update-available-symbolic",
    );
    //

    refresh_button.connect_clicked(clone!(
        #[weak]
        apt_retry_signal_action,
        #[weak]
        window_adw_view_stack,
        move |_| {
            match window_adw_view_stack.visible_child_name().unwrap().as_str() {
                "apt_update_page" => apt_retry_signal_action.activate(None),
                _ => {}
            }
        }
    ));
}
