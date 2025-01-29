set -v

if ! type wasm-pack > /dev/null; then
    echo "wasm-pack is not installed"
    cargo install wasm-pack
fi

if ! type basic-http-server > /dev/null; then
    echo "basic-http-server is not installed"
    cargo install basic-http-server
fi

wasm-pack build --target web --release -- 

dest_dir="../../../ivanceras.github.io/data-viewer"

mkdir -p $dest_dir;

cp index.html $dest_dir/index.html
cp style.css $dest_dir/style.css
cp reset.css $dest_dir/reset.css
cp -r pkg $dest_dir/

## Remove the ignore file on the pkg directory
rm $dest_dir/pkg/.gitignore
