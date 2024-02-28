#ifndef SERVER_H
#define SERVER_H

#include <unordered_map>

#include "httplib.h"
#include "json.hpp"
#include "product.h"

using Json = nlohmann::json;

class HttpServer {
private:
    httplib::Server server_;
    std::unordered_map<uint32_t, Product> products_;
    std::unordered_map<std::string, std::string> images_;
    uint32_t max_id_{0};

public:
    HttpServer();

    inline void Listen() {
        server_.listen("127.0.0.1", 8080);
    }
};

#endif