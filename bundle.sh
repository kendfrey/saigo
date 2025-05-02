shopt -s extglob
mkdir -p dist/x86_64-pc-windows-msvc dist/x86_64-unknown-linux-gnu
cp target/x86_64-pc-windows-msvc/release/*.exe model.safetensors model.txt dist/x86_64-pc-windows-msvc
cp -r html dist/x86_64-pc-windows-msvc
cp target/x86_64-unknown-linux-gnu/release/!(*.*) model.safetensors model.txt dist/x86_64-unknown-linux-gnu
cp -r html dist/x86_64-unknown-linux-gnu
cd dist
zip -r x86_64-pc-windows-msvc.zip x86_64-pc-windows-msvc
zip -r x86_64-unknown-linux-gnu.zip x86_64-unknown-linux-gnu