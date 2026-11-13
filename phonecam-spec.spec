Name:           phonecam-studio
Version:        0.1.0
Release:        1%{?dist}
Summary:        A modern GTK4 GUI wrapper for scrcpy and v4l2loopback

License:        MIT
URL:            https://github.com/phonecam/phonecam-studio

BuildRequires:  cargo rust gtk4-devel v4l-utils
Requires:       scrcpy v4l2loopback-dkms v4l-utils gtk4

%description
A modern GTK4 GUI wrapper for scrcpy and v4l2loopback to use phone camera on Linux.

%prep
# No prep needed for direct build

%build
cargo build --release

%install
install -Dm755 target/release/phonecam_studio %{buildroot}%{_bindir}/phonecam-studio

%files
%{_bindir}/phonecam-studio

%changelog
* Fri Nov 13 2026 Ahmed Al-Iraqi <ahmed@local> - 0.1.0-1
- Initial release
