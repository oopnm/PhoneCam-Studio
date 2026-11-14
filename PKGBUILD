# Maintainer: Ahmed Al-Iraqi <ahmed@local>
pkgname=phonecam-studio
pkgver=0.1.0
pkgrel=1
pkgdesc="A modern GTK4 GUI wrapper for scrcpy and v4l2loopback to use phone camera on Linux"
arch=('x86_64')
url="https://github.com/oopnm/PhoneCam-Studio"
license=('GPL3')
depends=('gtk4' 'scrcpy' 'v4l2loopback-dkms' 'v4l-utils' 'polkit')
makedepends=('cargo' 'rust')
# ملاحظة: شغّل makepkg من مجلد منفصل (وليس من جذر الريبو) لأن makepkg
# يستخدم مجلد src/ لفك المصدر وسيتعارض مع مجلد src/ الخاص بالمشروع.
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "PhoneCam-Studio-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release --locked
}

package() {
    cd "PhoneCam-Studio-$pkgver"
    install -Dm755 target/release/phonecam-studio "$pkgdir/usr/bin/phonecam-studio"
    install -Dm644 appimage/phonecam-studio.desktop "$pkgdir/usr/share/applications/phonecam-studio.desktop"
    install -Dm644 appimage/phonecam-studio.png "$pkgdir/usr/share/icons/hicolor/256x256/apps/phonecam-studio.png"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
