use gtk::glib;
use gtk::prelude::*;
use gtk::Box as GtkBox;
use gtk::{
    Application, ApplicationWindow, Button, CheckButton, DropDown, Entry, Label, Orientation,
    ScrolledWindow, TextBuffer, TextView,
};
use std::cell::RefCell;
use std::process::Stdio;
use std::rc::Rc;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};

const APP_ID: &str = "com.github.phonecam.studio";
const DEFAULT_DEVICE: &str = "/dev/video10";

/// رسائل تُرسل من مهام Tokio إلى واجهة GTK (تُعالَج فقط على الخيط الرئيسي).
enum UiEvent {
    Log(String),
    Running(bool),
}

type EventTx = mpsc::UnboundedSender<UiEvent>;

fn main() -> glib::ExitCode {
    // GTK يملك الخيط الرئيسي، لذلك يعمل Tokio في خيط خلفي منفصل.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("تعذر إنشاء Tokio runtime");
    let handle = rt.handle().clone();

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| build_ui(app, handle.clone()));
    // عند الخروج يُسقَط الـ runtime، ومعه تُقتل عمليات scrcpy (kill_on_drop).
    app.run()
}

fn build_ui(app: &Application, rt: Handle) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("PhoneCam Studio - Linux")
        .default_width(650)
        .default_height(600)
        .build();

    let main_box = GtkBox::new(Orientation::Vertical, 12);
    main_box.set_margin_start(16);
    main_box.set_margin_end(16);
    main_box.set_margin_top(16);
    main_box.set_margin_bottom(16);

    main_box.append(
        &Label::builder()
            .label("<b>PhoneCam Studio (Scrcpy + v4l2loopback)</b>")
            .use_markup(true)
            .xalign(0.0)
            .build(),
    );

    // مؤشر حالة البث
    let status_label = Label::builder().xalign(0.0).use_markup(true).build();
    set_status(&status_label, false);
    main_box.append(&status_label);

    let settings_box = GtkBox::new(Orientation::Vertical, 8);

    // كاميرا الهاتف (أمامية / خلفية)
    let facing_box = GtkBox::new(Orientation::Horizontal, 10);
    facing_box.append(&row_label("كاميرا الهاتف:"));
    let facing_dropdown = DropDown::new(
        Some(gtk::StringList::new(&["خلفية (Back)", "أمامية (Front)"])),
        gtk::Expression::NONE,
    );
    facing_box.append(&facing_dropdown);
    settings_box.append(&facing_box);

    // الدقة
    let res_box = GtkBox::new(Orientation::Horizontal, 10);
    res_box.append(&row_label("دقة البث (Resolution):"));
    let res_dropdown = DropDown::new(
        Some(gtk::StringList::new(&["1920x1080", "1280x720", "3840x2160 (4K)"])),
        gtk::Expression::NONE,
    );
    res_dropdown.set_selected(0);
    res_box.append(&res_dropdown);
    settings_box.append(&res_box);

    // جهاز V4L2
    let device_box = GtkBox::new(Orientation::Horizontal, 10);
    device_box.append(&row_label("جهاز V4L2:"));
    let device_entry = Entry::builder().text(DEFAULT_DEVICE).build();
    device_box.append(&device_entry);
    settings_box.append(&device_box);

    let audio_check = CheckButton::builder()
        .label("تضمين صوت المايكروفون (Audio Forwarding)")
        .active(false)
        .build();
    settings_box.append(&audio_check);
    main_box.append(&settings_box);

    // أزرار التحكم
    let action_box = GtkBox::new(Orientation::Horizontal, 10);
    let modprobe_btn = Button::builder()
        .label("1. تفعيل درايفر النواة (Modprobe)")
        .build();
    let start_btn = Button::builder().label("2. تشغيل الكاميرا 🟢").build();
    let stop_btn = Button::builder().label("إيقاف البث 🛑").build();
    action_box.append(&modprobe_btn);
    action_box.append(&start_btn);
    action_box.append(&stop_btn);
    main_box.append(&action_box);

    // السجلات
    main_box.append(
        &Label::builder()
            .label("<b>سجلات النظام والأخطاء الفورية:</b>")
            .use_markup(true)
            .xalign(0.0)
            .build(),
    );
    let text_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    let text_buffer = text_view.buffer();
    main_box.append(
        &ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&text_view)
            .vexpand(true)
            .build(),
    );

    window.set_child(Some(&main_box));

    // --- قناة الأحداث: Tokio -> GTK ---
    // tokio::sync::mpsc لا يحتاج runtime لانتظار الاستقبال، فيمكن انتظاره
    // مباشرة داخل حلقة GLib الرئيسية بدون أي استدعاء GTK من خيط آخر.
    let (tx, mut rx) = mpsc::unbounded_channel::<UiEvent>();

    // مرسل الإيقاف للجلسة الحالية (None = لا يوجد بث).
    let stop_slot: Rc<RefCell<Option<oneshot::Sender<()>>>> = Rc::new(RefCell::new(None));

    {
        let buffer = text_buffer.clone();
        let status = status_label.clone();
        let start = start_btn.clone();
        let slot = stop_slot.clone();
        glib::spawn_future_local(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    UiEvent::Log(text) => append_log(&buffer, &text),
                    UiEvent::Running(running) => {
                        if !running {
                            slot.borrow_mut().take();
                        }
                        start.set_sensitive(!running);
                        set_status(&status, running);
                    }
                }
            }
        });
    }

    // --- ربط الأحداث ---

    // 1. تفعيل v4l2loopback (رقم الجهاز يؤخذ من حقل الإدخال)
    {
        let tx = tx.clone();
        let rt = rt.clone();
        let entry = device_entry.clone();
        modprobe_btn.connect_clicked(move |_| {
            let device = entry.text().to_string();
            match parse_video_nr(&device) {
                Some(nr) => {
                    rt.spawn(load_module(nr, tx.clone()));
                }
                None => log(
                    &tx,
                    format!("❌ مسار غير صالح: «{device}». الصيغة المطلوبة: /dev/videoN\n"),
                ),
            }
        });
    }

    // 2. بدء البث
    {
        let tx = tx.clone();
        let rt = rt.clone();
        let slot = stop_slot.clone();
        let facing_dd = facing_dropdown.clone();
        let res_dd = res_dropdown.clone();
        let entry = device_entry.clone();
        let audio = audio_check.clone();
        start_btn.connect_clicked(move |_| {
            if slot.borrow().is_some() {
                return; // يوجد بث شغال أصلاً
            }
            let device = entry.text().to_string();
            if parse_video_nr(&device).is_none() {
                log(
                    &tx,
                    format!("❌ مسار غير صالح: «{device}». الصيغة المطلوبة: /dev/videoN\n"),
                );
                return;
            }
            let facing = if facing_dd.selected() == 1 { "front" } else { "back" };
            let size = match res_dd.selected() {
                1 => "1280x720",
                2 => "3840x2160",
                _ => "1920x1080",
            };

            let mut args = vec![
                "--video-source=camera".to_string(),
                format!("--camera-facing={facing}"),
                format!("--camera-size={size}"),
                format!("--v4l2-sink={device}"),
                // لا نافذة معاينة (يُبقي الصوت شغال إذا فُعّل)
                "--no-video-playback".to_string(),
            ];
            if audio.is_active() {
                args.push("--audio-source=mic".to_string());
            } else {
                args.push("--no-audio".to_string());
            }

            log(
                &tx,
                format!("> تشغيل scrcpy (كاميرا: {facing}, دقة: {size}, الجهاز: {device})...\n"),
            );

            let (stop_tx, stop_rx) = oneshot::channel();
            *slot.borrow_mut() = Some(stop_tx);
            let _ = tx.send(UiEvent::Running(true));
            rt.spawn(run_scrcpy(args, stop_rx, tx.clone()));
        });
    }

    // 3. إيقاف البث (فقط الجلسة التي شغلها التطبيق، بدون pkill)
    {
        let tx = tx.clone();
        let slot = stop_slot.clone();
        stop_btn.connect_clicked(move |_| match slot.borrow_mut().take() {
            Some(stop_tx) => {
                let _ = stop_tx.send(());
            }
            None => log(&tx, "لا يوجد بث نشط حالياً.\n".to_string()),
        });
    }

    window.present();
}

/// تحميل وحدة النواة عبر pkexec (نافذة مصادقة رسومية؛ sudo لا يعمل بدون طرفية).
async fn load_module(nr: u32, tx: EventTx) {
    let nr_arg = format!("video_nr={nr}");
    log(
        &tx,
        format!("> pkexec modprobe v4l2loopback ({nr_arg}) — قد تظهر نافذة إدخال كلمة المرور...\n"),
    );

    let result = Command::new("pkexec")
        .args([
            "modprobe",
            "v4l2loopback",
            "devices=1",
            &nr_arg,
            "card_label=PhoneCam",
            "exclusive_caps=1",
        ])
        .stdin(Stdio::null())
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => {
            log(&tx, format!("✓ تم تفعيل v4l2loopback على /dev/video{nr}\n"));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let hint = match out.status.code() {
                Some(126) | Some(127) => " (أُلغيت المصادقة أو رُفضت)",
                _ => "",
            };
            log(&tx, format!("❌ فشل تفعيل الدرايفر{hint}: {}\n", stderr.trim()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => log(
            &tx,
            "❌ الأمر pkexec غير موجود. ثبّت polkit، أو نفّذ يدوياً: sudo modprobe v4l2loopback\n"
                .to_string(),
        ),
        Err(e) => log(&tx, format!("❌ تعذر تنفيذ pkexec: {e}\n")),
    }
}

/// تشغيل scrcpy ومراقبته حتى ينتهي أو يُطلب إيقافه.
async fn run_scrcpy(args: Vec<String>, mut stop_rx: oneshot::Receiver<()>, tx: EventTx) {
    let spawned = Command::new("scrcpy")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();

    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "❌ scrcpy غير مثبت أو غير موجود في PATH.\n".to_string()
            } else {
                format!("❌ فشل تشغيل scrcpy: {e}\n")
            };
            log(&tx, msg);
            let _ = tx.send(UiEvent::Running(false));
            return;
        }
    };

    let out_task = child
        .stdout
        .take()
        .map(|s| tokio::spawn(pump_lines(s, "", tx.clone())));
    let err_task = child
        .stderr
        .take()
        .map(|s| tokio::spawn(pump_lines(s, "⚠️ ", tx.clone())));

    tokio::select! {
        status = child.wait() => match status {
            Ok(s) if s.success() => log(&tx, "■ انتهى scrcpy.\n".to_string()),
            Ok(s) => log(&tx, format!("❌ توقف scrcpy بشكل غير متوقع ({s}). راجع السجل أعلاه.\n")),
            Err(e) => log(&tx, format!("❌ خطأ أثناء انتظار scrcpy: {e}\n")),
        },
        _ = &mut stop_rx => {
            // kill() يقتل العملية وينتظرها فلا تبقى zombie
            match child.kill().await {
                Ok(()) => log(&tx, "🛑 تم إيقاف البث.\n".to_string()),
                Err(e) => log(&tx, format!("خطأ أثناء إيقاف scrcpy: {e}\n")),
            }
        }
    }

    // اترك قارئي المخرجات ينهون ما تبقى قبل إعلان التوقف
    for task in [out_task, err_task].into_iter().flatten() {
        let _ = task.await;
    }
    let _ = tx.send(UiEvent::Running(false));
}

async fn pump_lines<R: AsyncRead + Unpin>(reader: R, prefix: &'static str, tx: EventTx) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        log(&tx, format!("{prefix}{line}\n"));
    }
}

fn log(tx: &EventTx, text: String) {
    let _ = tx.send(UiEvent::Log(text));
}

fn parse_video_nr(device: &str) -> Option<u32> {
    device.trim().strip_prefix("/dev/video")?.parse().ok()
}

fn row_label(text: &str) -> Label {
    Label::builder().label(text).xalign(0.0).width_request(140).build()
}

fn set_status(label: &Label, running: bool) {
    label.set_markup(if running {
        "<b>الحالة: 🟢 متصل</b>"
    } else {
        "<b>الحالة: ⚪ متوقف</b>"
    });
}

fn append_log(buffer: &TextBuffer, text: &str) {
    let mut end = buffer.end_iter();
    buffer.insert(&mut end, text);
}
