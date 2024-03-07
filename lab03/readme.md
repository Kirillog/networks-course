Install boost 1.74:
```
sudo apt-get install libboost-all-dev
```
Optionally install **cmake** and **c++** compiler, if there no on your system.

To run server build project firstly
```
( cd build; cmake .. -DCMAKE_BUILD_TYPE=Release )
```
Then run according to usage:
```
./build/server 127.0.0.1 8080 10
./build/client 127.0.0.1 8080 a.txt
```