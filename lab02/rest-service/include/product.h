#ifndef PRODUCT_H
#define PRODUCT_H

#include <cstdint>
#include <string>

#include "json.hpp"

using Json = nlohmann::json;

struct Product {

    static constexpr const char* ID = "id";
    static constexpr const char* NAME = "name";
    static constexpr const char* DESCRIPTION = "description";
    static constexpr const char* IMAGE = "icon";

    uint32_t id;
    std::string name;
    std::string description;
    std::string image;

    inline Json ToJSON() const {
        Json product;
        product[ID] = id;
        product[NAME] = name;
        product[DESCRIPTION] = description;
        product[IMAGE] = image;
        return product;
    }
};

#endif