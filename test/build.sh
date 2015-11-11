arg="$1"
type="debug"

if [ -z "$arg" ]
then
    type="debug"
else
    type="release"
fi

if [ ! -d "libs" ]
then
    echo "mkdir libs"
    mkdir libs
fi

cd ../

echo $type

if [ "$type" == "debug" ]
then
    echo "debug version"
    cargo build
else
    echo "release version"
    cargo build --$TYPE
fi

cd test/
cp ../target/$type/libatpp.rlib ./libs/
cp ../target/$type/deps/* ./libs/
rustc server.rs -L ./libs/
