#include <boost/asio/buffer.hpp>
#include <boost/asio/error.hpp>
#include <boost/asio/ip/address.hpp>
#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/post.hpp>
#include <boost/asio/thread_pool.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/core/error.hpp>
#include <boost/beast/core/multi_buffer.hpp>
#include <boost/beast/http.hpp>
#include <boost/beast/http/dynamic_body.hpp>
#include <boost/beast/http/message.hpp>
#include <boost/beast/http/status.hpp>
#include <boost/beast/http/string_body.hpp>
#include <boost/beast/version.hpp>
#include <boost/config.hpp>
#include <boost/system/error_code.hpp>

#include <boost/archive/text_iarchive.hpp>
#include <boost/archive/text_oarchive.hpp>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <mutex>
#include <ostream>
#include <sstream>
#include <string>
#include <thread>
#include <unordered_set>
#include <variant>

#include "serialize.hpp"
#include "uri.hpp"

namespace beast = boost::beast;
namespace http = beast::http;
namespace net = boost::asio;
using tcp = boost::asio::ip::tcp;

using namespace std::filesystem;

void fail(beast::error_code ec, char const *what) {
  std::cerr << what << ": " << ec.message() << "\n";
}

template <class T> struct Cached {
  std::string etag;
  T response;
};

class ProxyServer {
private:
  const net::ip::address address_ = net::ip::make_address("127.0.0.1");
  const unsigned short port_ = 8080;
  const uint32_t concurrency_level_ = std::thread::hardware_concurrency();

private:
  template <typename T>
  void WriteResponse(tcp::socket &socket, http::response<T> &&msg,
                     beast::error_code ec) {

    beast::http::write(socket, msg, ec);
  }

  void HandleSession(tcp::socket &socket) {
    beast::error_code ec;

    beast::flat_buffer buffer;

    for (;;) {
      http::request<http::string_body> req;
      http::read(socket, buffer, req, ec);
      if (ec == http::error::end_of_stream) {
        break;
      }
      if (ec) {
        return fail(ec, "read");
      }

      auto common_response = HandleRequest(std::move(req));

      if (common_response.index() == 0) {
        WriteResponse(socket, std::move(std::get<0>(common_response)), ec);
      } else if (common_response.index() == 1) {
        WriteResponse(socket, std::move(std::get<1>(common_response)), ec);
      }

      if (ec) {
        return fail(ec, "write");
      }
    }

    socket.shutdown(tcp::socket::shutdown_send, ec);
  }

  template <typename T>
  static void SetErrorMessage(const http::request<http::string_body> &req,
                              http::response<T> &res, std::string &&body) {
    res.set(http::field::server, BOOST_BEAST_VERSION_STRING);
    res.set(http::field::content_type, "text/html");
    res.keep_alive(req.keep_alive());
    res.body() = body;
    res.prepare_payload();
  }

  std::variant<http::response<http::dynamic_body>,
               http::response<http::string_body>>
  HandleRequest(http::request<http::string_body> &&req) {

    if (req.method() != http::verb::get && req.method() != http::verb::post) {
      http::response<http::string_body> res{http::status::bad_request,
                                            req.version()};
      SetErrorMessage(req, res, "Unknown HTTP-method");
      return res;
    }

    uri url = uri(req.target().to_string());

    auto host = url.get_host();
    if (blacklist_.count(host) > 0) {
      http::response<http::string_body> res{http::status::bad_request,
                                            req.version()};
      SetErrorMessage(req, res, "Banned host " + host);
      return res;
    }
    auto target = url.get_path();
    if (target.back() != '/') {
      target = target + "/";
    }
    std::cerr << host << " " << target << "\n";

    tcp::resolver resolver(ioc);
    beast::tcp_stream stream(ioc);
    auto const results = resolver.resolve(host, "80");
    beast::error_code ec;

    stream.connect(results, ec);
    if (ec) {
      http::response<http::string_body> res{http::status::bad_request,
                                            req.version()};
      SetErrorMessage(req, res, "Failed to connect to host " + host);
      return res;
    }

    std::string key = host + "/" + target;
    auto it = runtime_cache_.find(key);

    req.target() = target;
    req.set(http::field::host, host);
    auto st = it == runtime_cache_.end() ? "" : it->second.etag;
    req.set(http::field::if_none_match, st);
    req.prepare_payload();

    http::write(stream, req);

    beast::flat_buffer buffer;
    http::response<http::dynamic_body> res;

    http::read(stream, buffer, res);
    auto etag_field = res.base().find(http::field::etag);

    if (res.result() == http::status::not_modified) {
      std::lock_guard guard{journal_mutex_};
      journal_ << "Cached: " << host << " " << res.result_int() << "\n";
      journal_.flush();
      return it->second.response;
    } else if (etag_field != res.base().end()) {
      std::lock_guard guard{cache_mutex_};
      ++changes_count_;
      runtime_cache_[key] =
          CachedResponse{etag_field->value().to_string(), res};
    }
    LogOnDisk(host, res.result_int());
    if (changes_count_ > 100) {
      WriteCacheOnDisk();
    }
    return res;
  }

  void LogOnDisk(const std::string &host, int result) {
    std::lock_guard guard{journal_mutex_};
    journal_ << host << " " << result << "\n";
    journal_.flush();
  }

  void WriteCacheOnDisk() {
    std::lock_guard guard{cache_mutex_};
    std::ofstream ofs(path_.string() + "cache.bin");
    ofs << bits(runtime_cache_);
    changes_count_ = 0;
    ofs.close();
  }

  void ReadCacheFromDisk() {
    std::lock_guard guard{cache_mutex_};
    std::ifstream ifs(path_.string() + "cache.bin");
    ifs >> bits(runtime_cache_);
    ifs.close();
  }

public:
  ProxyServer(path &&pth, std::unordered_set<std::string> &&blacklist) {
    path_ = pth;
    create_directories(path_);

    journal_ = std::ofstream(path_.string() + "log.txt");
    blacklist_ = blacklist;
    ReadCacheFromDisk();
  }

  void Run() {

    tcp::acceptor acceptor{ioc, {address_, port_}};
    for (;;) {
      tcp::socket socket{ioc};

      acceptor.accept(socket);

      net::post(pool, std::bind(&ProxyServer::HandleSession, this,
                                std::move(socket)));
    }
  }

private:
  std::ofstream journal_;
  std::mutex journal_mutex_, cache_mutex_;
  path path_;

  using CachedResponse = Cached<http::response<http::dynamic_body>>;

  std::unordered_set<std::string> blacklist_;
  std::unordered_map<std::string, CachedResponse> runtime_cache_;
  uint64_t changes_count_{0};

  net::io_context ioc{1};
  net::thread_pool pool{concurrency_level_};
};

std::unordered_set<std::string> blacklist(const path &file) {
  std::ifstream fin(file);
  std::string banned_host;
  std::unordered_set<std::string> blacklist;
  while (fin >> banned_host) {
    blacklist.insert(banned_host);
  }
  return blacklist;
}

int main(int argc, char *argv[]) {
  try {
    if (argc > 3) {
      std::cerr << "Usage: proxy_server [directory_path] [blacklist]\n";
      return EXIT_FAILURE;
    }
    path directory = argc > 1 ? argv[1] : temp_directory_path();
    directory += std::filesystem::path::preferred_separator;
    std::string blacklist_file = argc > 2 ? argv[2] : "blacklist.cfg";
    ProxyServer server(std::move(directory),
                       blacklist(directory.string() + blacklist_file));
    server.Run();
  } catch (const std::exception &e) {
    std::cerr << "Error: " << e.what() << std::endl;
    return EXIT_FAILURE;
  }
}