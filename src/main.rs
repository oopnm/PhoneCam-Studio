use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, Button, CheckButton, DropDown, Entry, Label, Orientation,
    ScrolledWindow, TextBuffer, TextView,
};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const APP_ID: &str = "com.github.phonecam.studio";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("PhoneCam Studio - Linux")
        .default_width(650)
        .default_height(600)
        .build();

    let main_box = Box::new(Orientation::Vertical, 12);
    main_box.set_margin_start(16);
    main_box.set_margin_end(16);
    main_box.set_margin_top(16);
    main_box.set_margin_bottom(16);

    // عنوان التطبيق
    let title_label = Label::builder()
        .label("<b>PhoneCam Studio (Scrcpy + v4l2loopback)</b>")
        .use_markup(true)
        .xalign(0.0)
        .build();
    main_box.append(&title_label);

    // قسم الإعدادات
    let settings_box = Box::new(Orientation::Vertical, 8);

    // اختيار الكاميرا (أمامية / خلفية) مع تحديد النوع لتجنب أخطاء الكومبايلر
    let facing_box = Box::new(Orientation::Horizontal, 10);
    facing_box.append(
        &Label::builder()
            .label("كاميرا الهاتف:")
            .xalign(0.0)
            .width_request(140)
            .build(),
    );
    let facing_items = &["خلفية (Back)", "أمامية (Front)"];
    let facing_model = gtk::StringList::new(facing_items);
    let facing_dropdown = DropDown::new(Some(&facing_model), None::<&gtk::Expression>);
    facing_box.append(&facing_dropdown);
    settings_box.append(&facing_box);

    // اختيار الدقة (Resolution)
    let res_box = Box::new(Orientation::Horizontal, 10);
    res_box.append(
        &Label::builder()
            .label("دقة البث (Resolution):")
            .xalign(0.0)
            .width_request(140)
            .build(),
    );
    let res_items = &["1920x1080", "1280x720", "3840x2160 (4K)"];
    let res_model = gtk::StringList::new(res_items);
    let res_dropdown = DropDown::new(Some(&res_model), None::<&gtk::Expression>);
    res_dropdown.set_selected(0);
    res_box.append(&res_dropdown);
    settings_box.append(&res_box);

    // رقم جهاز V4L2
    let device_box = Box::new(Orientation::Horizontal, 10);
    device_box.append(
        &Label::builder()
            .label("رقم جهاز V4L2:")
            .xalign(0.0)
            .width_request(140)
            .build(),
    );
    let device_entry = Entry::builder().text("/dev/video10").build();
    device_box.append(&device_entry);
    settings_box.append(&device_box);

    // خيار نقل الصوت
    let audio_check = CheckButton::builder()
        .label("تضمين صوت المايكروفون (Audio Forwarding)")
        .active(false)
        .build();
    settings_box.append(&audio_check);

    main_box.append(&settings_box);

    // أزرار التحكم
    let action_box = Box::new(Orientation::Horizontal, 10);
    let modprobe_btn = Button::builder()
        .label("1. تفعيل درايفر النواة (Modprobe)")
        .build();
    let start_btn = Button::builder().label("2. تشغيل الكاميرا 🟢").build();
    let stop_btn = Button::builder().label("إيقاف البث 🛑").build();

    action_box.append(&modprobe_btn);
    action_box.append(&start_btn);
    action_box.append(&stop_btn);
    main_box.append(&action_box);

    // منطقة السجلات والأخطاء
    let log_label = Label::builder()
        .label("<b>سجلات النظام والأخطاء الفورية:</b>")
        .use_markup(true)
        .xalign(0.0)
        .build();
    main_box.append(&log_label);

    let text_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    let text_buffer = text_view.buffer();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&text_view)
        .vexpand(true)
        .build();
    main_box.append(&scrolled_window);

    window.set_child(Some(&main_box));

    // --- ربط الأحداث ---

    // 1. تفعيل v4l2loopback
    let tb_clone = text_buffer.clone();
    modprobe_btn.connect_clicked(move |_| {
        let tb = tb_clone.clone();
        std::thread::spawn(move || {
            let output = std::process::Command::new("sudo")
                .args([
                    "modprobe",
                    "v4l2loopback",
                    "devices=1",
                    "video_nr=10",
                    "card_label=PhoneCam",
                    "exclusive_caps=1",
                ])
                .output();

            glib::idle_add_local(move || {
                match output {
                    Ok(res) => {
                        if res.status.success() {
                            append_log(&tb, "✓ تم تفعيل درايفر v4l2loopback بنجاح على /dev/video10\n");
                        } else {
                            let err = String::from_utf8_lossy(&res.stderr);
                            append_log(&tb, &format!("❌ فشل تفعيل الدرايفر: {}\n", err));
                        }
                    }
                    Err(e) => {
                        append_log(&tb, &format!("❌ خطأ بتنفيذ الأمر (تحقق من صلاحيات sudo): {}\n", e));
                    }
                }
                glib::ControlFlow::Break
            });
        });
    });

    // 2. بدء البث
    let facing_dp_clone = facing_dropdown.clone();
    let res_dp_clone = res_dropdown.clone();
    let dev_entry_clone = device_entry.clone();
    let audio_chk_clone = audio_check.clone();
    let tb_start = text_buffer.clone();

    start_btn.connect_clicked(move |_| {
        let facing = if facing_dp_clone.selected() == 1 {
            "front"
        } else {
            "back"
        };
        let resolution = match res_dp_clone.selected() {
            1 => "1280x720",
            2 => "3840x2160",
            _ => "1920x1080",
        };
        let device = dev_entry_clone.text().to_string();
        let audio_enabled = audio_chk_clone.is_active();
        let tb = tb_start.clone();

        append_log(
            &tb,
            &format!(
                "> جاري بدء تشغيل scrcpy (كاميرا: {}, دقة: {}, الجهاز: {})...\n",
                facing, resolution, device
            ),
        );

        tokio::spawn(async move {
            let mut args = vec![
                "--video-source=camera",
                match facing {
                    "front" => "--camera-facing=front",
                    _ => "--camera-facing=back",
                },
                match resolution {
                    "1280x720" => "--camera-size=1280x720",
                    "3840x2160" => "--camera-size=3840x2160",
                    _ => "--camera-size=1920x1080",
                },
                // تمرير كزعنفة v4l2 باستخدام String لتجنب مشاكل lifetimes
                Box::leak(format!("--v4l2-sink={}", device).into_boxed_str()) as &str,
                "--no-window",
            ];

            if !audio_enabled {
                args.push("--no-audio");
            } else {
                args.push("--audio-source=mic");
            }

            let mut cmd = Command::new("scrcpy");
            cmd.args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    if let Some(out) = stdout {
                        let mut reader = BufReader::new(out).lines();
                        let tb_inner = tb.clone();
                        tokio::spawn(async move {
                            while let Ok(Some(line)) = reader.next_line().await {
                                let tb_c = tb_inner.clone();
                                glib::idle_add_local(move || {
                                    append_log(&tb_c, &format!("{}\n", line));
                                    glib::ControlFlow::Break
                                });
                            }
                        });
                    }

                    if let Some(err) = stderr {
                        let mut reader = BufReader::new(err).lines();
                        let tb_inner = tb.clone();
                        tokio::spawn(async move {
                            while let Ok(Some(line)) = reader.next_line().await {
                                let tb_c = tb_inner.clone();
                                glib::idle_add_local(move || {
                                    append_log(&tb_c, &format!("⚠️ [Log] {}\n", line));
                                    glib::ControlFlow::Break
                                });
                            }
                        });
                    }
                }
                Err(e) => {
                    let tb_c = tb.clone();
                    glib::idle_add_local(move || {
                        append_log(
                            &tb_c,
                            &format!("❌ فشل تشغيل أمر scrcpy: تأكد من تثبيته. الخطأ: {}\n", e),
                        );
                        glib::ControlFlow::Break
                    });
                }
            }
        });
    });

    // 3. إيقاف البث
    let tb_stop = text_buffer.clone();
    stop_btn.connect_clicked(move |_| {
        let tb = tb_stop.clone();
        std::thread::spawn(move || {
            let output = std::process::Command::new("pkill").arg("scrcpy").output();

            glib::idle_add_local(move || {
                match output {
                    Ok(_) => append_log(&tb, "🛑 تم إيقاف جميع جلسات scrcpy النشطة.\n"),
                    Err(e) => append_log(&tb, &format!("خطأ أثناء إيقاف العمليات: {}\n", e)),
                }
                glib::ControlFlow::Break
            });
        });
    });

    window.present();
}

fn append_log(buffer: &TextBuffer, text: &str) {
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, text);
}