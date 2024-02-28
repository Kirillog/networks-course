#include "server.h"
#include "product.h"
#include <httplib.h>

HttpServer::HttpServer() {
    server_.Get("/product/:id", [&](const httplib::Request &req, httplib::Response &res) {
        uint32_t product_id = static_cast<uint32_t>(std::stoul(req.path_params.at(Product::ID)));
        std::cerr << "Get " << product_id << "\n";
        auto product = this->products_.find(product_id);
        if (product == this->products_.end()) {
            res.status = httplib::StatusCode::NotFound_404;
        } else {
            res.status = httplib::StatusCode::OK_200;
            res.set_content(product->second.ToJSON().dump(), "application/json");
        }
        return res.status;
    });

    server_.Post("/product", [&](const httplib::Request &req, httplib::Response &res) {
        std::cerr << "Post"
                  << "\n";
        Json data = Json::parse(req.body);
        uint32_t product_id = max_id_++;
        if (!data.contains(Product::NAME) || !data.contains(Product::DESCRIPTION)) {
            res.status = httplib::StatusCode::BadRequest_400;
        } else {
            auto product = Product{product_id, data[Product::NAME], data[Product::DESCRIPTION], ""};
            this->products_[product_id] = product;
            res.set_content(product.ToJSON().dump(), "application/json");
            res.status = httplib::StatusCode::OK_200;
        }
        return res.status;
    });

    server_.Put("/product/:id", [&](const httplib::Request &req, httplib::Response &res) {
        uint32_t product_id = static_cast<uint32_t>(std::stoul(req.path_params.at(Product::ID)));
        std::cerr << "Put " << product_id << "\n";
        auto product = this->products_.find(product_id);
        if (product == this->products_.end()) {
            res.status = httplib::StatusCode::NotFound_404;
        } else {
            res.status = httplib::StatusCode::OK_200;
            Json data = Json::parse(req.body);
            if (data.contains(Product::NAME)) {
                product->second.name = data[Product::NAME];
            }
            if (data.contains(Product::DESCRIPTION)) {
                product->second.description = data[Product::DESCRIPTION];
            }
            res.set_content(product->second.ToJSON().dump(), "application/json");
        }
        return res.status;
    });

    server_.Delete("/product/:id", [&](const httplib::Request &req, httplib::Response &res) {
        uint32_t product_id = static_cast<uint32_t>(std::stoul(req.path_params.at(Product::ID)));
        std::cerr << "Delete " << product_id << "\n";
        auto product = this->products_.find(product_id);
        if (product == this->products_.end()) {
            res.status = httplib::StatusCode::NotFound_404;
        } else {
            res.status = httplib::StatusCode::OK_200;
            res.set_content(product->second.ToJSON().dump(), "application/json");
            this->products_.erase(product);
        }
        return res.status;
    });

    server_.Get("/products", [&](const httplib::Request &, httplib::Response &res) {
        res.status = httplib::StatusCode::OK_200;
        std::cerr << "Getting all"
                  << "\n";
        std::vector<Json> products;
        for (auto &[id, product] : this->products_) {
            products.emplace_back(product.ToJSON());
        }
        res.set_content(Json(products).dump(), "application/json");
    });

    server_.Post("/product/:id/image", [&](const httplib::Request &req, httplib::Response &res) {
        uint32_t product_id = static_cast<uint32_t>(std::stoul(req.path_params.at(Product::ID)));
        std::cerr << "Post image to product " << product_id << "\n";
        auto product = this->products_.find(product_id);
        if (product == this->products_.end()) {
            res.status = httplib::StatusCode::NotFound_404;
            return res.status;
        }
        if (!req.has_file("icon")) {
            res.status = httplib::StatusCode::BadRequest_400;
            return res.status;
        }
        auto file = req.get_file_value(Product::IMAGE);
        if (file.content_type != "image/png") {
            res.status = httplib::StatusCode::BadRequest_400;
            return res.status;
        }
        product->second.image = file.filename;
        this->images_[file.filename] = file.content;
        res.status = httplib::StatusCode::OK_200;
        return res.status;
    });

    server_.Get("/product/:id/image", [&](const httplib::Request &req, httplib::Response &res) {
        uint32_t product_id = static_cast<uint32_t>(std::stoul(req.path_params.at(Product::ID)));
        std::cerr << "Get " << product_id << " image\n";
        auto product = this->products_.find(product_id);
        if (product == this->products_.end() || product->second.image.empty()) {
            res.status = httplib::StatusCode::NotFound_404;
            return res.status;
        }
        res.set_content(images_[product->second.image], "image/png");
        return res.status = httplib::StatusCode::OK_200;
    });
}
