%global debug_package %{nil}

Name:           phonecam-studio
Version:        0.1.0
Release:        1%{?dist}
Summary:        A modern GTK4 GUI wrapper for scrcpy and v4l2loopback

License:        GPL-3.0-or-later
URL:            https://github.com/oopnm/PhoneCam-Studio
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo rust gcc gtk4-devel pkgconfig
Requires:       gtk4 v4l-utils polkit
# غير متوفرة في مستودعات Fedora الرسمية (RPM Fusion / COPR)
Recommends:     scrcpy akmod-v4l2loopback

%description
A modern GTK4 GUI wrapper for scrcpy and v4l2loopback to use phone camera on Linux.

%prep
%autosetup -n PhoneCam-Studio-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/phonecam-studio %{buildroot}%{_bindir}/phonecam-studio
install -Dm644 appimage/phonecam-studio.desktop %{buildroot}%{_datadir}/applications/phonecam-studio.desktop
install -Dm644 appimage/phonecam-studio.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/phonecam-studio.png

%files
%license LICENSE
%{_bindir}/phonecam-studio
%{_datadir}/applications/phonecam-studio.desktop
%{_datadir}/icons/hicolor/256x256/apps/phonecam-studio.png

%changelog
* Fri Nov 13 2026 Ahmed Al-Iraqi <ahmed@local> - 0.1.0-1
- Initial release
