# Maintainer: Ahmed Al-Iraqi <ahmed@local>
pkgname=phonecam-studio
pkgver=0.1.0
pkgrel=1
pkgdesc="A modern GTK4 GUI wrapper for scrcpy and v4l2loopback to use phone camera on Linux"
arch=('x86_64')
url="https://github.com/phonecam/phonecam-studio"
license=('MIT')
depends=('gtk4' 'scrcpy' 'v4l2loopback-dkms' 'v4l-utils')
makedepends=('cargo' 'rust')

build() {
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release --all-targets
}

package() {
    install -Dm755 target/release/phonecam_studio "$pkgdir/usr/bin/phonecam-studio"
}
