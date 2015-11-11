cd ../
if [ "$1"x == "debug"x ]
then
    echo "debug version"
    cargo build
else
    echo "release version"
    cargo build --release
fi

cd test/
cp ../target/$1/libatpp.rlib ./lib/
rustc   send.rs -L ./lib/
rustc server.rs -L ./lib/
