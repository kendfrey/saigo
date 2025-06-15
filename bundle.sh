shopt -s extglob
export VERSION=$(grep -m 1 "^version = " < Cargo.toml | grep -oP [\\d.]+)
mkdir -p dist/saigo-v$VERSION-x86_64-pc-windows-msvc dist/saigo-v$VERSION-x86_64-unknown-linux-gnu
cp target/x86_64-pc-windows-msvc/release/*.exe model.safetensors model.txt dist/saigo-v$VERSION-x86_64-pc-windows-msvc
cp -r html dist/saigo-v$VERSION-x86_64-pc-windows-msvc
cp target/x86_64-unknown-linux-gnu/release/!(*.*) model.safetensors model.txt dist/saigo-v$VERSION-x86_64-unknown-linux-gnu
cp -r html dist/saigo-v$VERSION-x86_64-unknown-linux-gnu
cd dist
zip -r saigo-v$VERSION-x86_64-pc-windows-msvc.zip saigo-v$VERSION-x86_64-pc-windows-msvc
zip -r saigo-v$VERSION-x86_64-unknown-linux-gnu.zip saigo-v$VERSION-x86_64-unknown-linux-gnu