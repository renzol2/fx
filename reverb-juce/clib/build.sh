cargo build --release
cbindgen -d --lang c++ -o moorer_verb.hpp .
clang++ --std=c++1z --stdlib=libc++ -L ./target/release -l fx_clib test.cpp
