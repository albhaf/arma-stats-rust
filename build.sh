#TODO: enable linux build when it's supported
#cargo build --target=x86_64-unknown-linux-gnu --release

export OPENSSL_LIB_DIR="/usr/i686-w64-mingw32/bin"
export OPENSSL_INCLUDE_DIR="/usr/i686-w64-mingw32/include"

cargo build --target=i686-pc-windows-gnu --release --verbose

