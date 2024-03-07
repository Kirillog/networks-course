//
// See
// https://github.com/boostorg/beast/blob/develop/example/http/server/sync/http_server_sync.cpp
//

#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/post.hpp>
#include <boost/asio/thread_pool.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/core/error.hpp>
#include <boost/beast/http.hpp>
#include <boost/beast/http/message.hpp>
#include <boost/beast/version.hpp>
#include <boost/config.hpp>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <string>
#include <thread>
#include <variant>

namespace beast = boost::beast;
namespace http = beast::http;
namespace net = boost::asio;
using tcp = boost::asio::ip::tcp;

void fail(beast::error_code ec, char const *what) {
  std::cerr << what << ": " << ec.message() << "\n";
}

std::variant<http::response<http::file_body>, http::response<http::string_body>>
handle_request(http::request<http::string_body> &&req) {

  auto set_error_message = [&](http::response<http::string_body> &res,
                               std::string &&body) {
    res.set(http::field::server, BOOST_BEAST_VERSION_STRING);
    res.set(http::field::content_type, "text/html");
    res.keep_alive(req.keep_alive());
    res.body() = body;
    res.prepare_payload();
  };
  if (req.method() != http::verb::get) {
    http::response<http::string_body> res{http::status::bad_request,
                                          req.version()};
    set_error_message(res, "Unknown HTTP-method");
    return res;
  }

  std::string path = req.target().to_string();
  beast::error_code ec;
  http::file_body::value_type body;
  body.open(path.c_str(), beast::file_mode::scan, ec);

  if (ec) {
    if (ec == beast::errc::no_such_file_or_directory) {
      http::response<http::string_body> res{http::status::not_found,
                                            req.version()};
      set_error_message(res, "The resource '" + std::string(req.target()) +
                                 "' was not found.");
      return res;

    } else {
      http::response<http::string_body> res{http::status::internal_server_error,
                                            req.version()};
      set_error_message(res, "An error occurred: '" +
                                 std::string(ec.message()) + "'");
      return res;
    }
  }

  auto const size = body.size();

  http::response<http::file_body> res{
      std::piecewise_construct, std::make_tuple(std::move(body)),
      std::make_tuple(http::status::ok, req.version())};
  res.set(http::field::server, BOOST_BEAST_VERSION_STRING);
  res.set(http::field::content_type, "application/text");
  res.content_length(size);
  res.keep_alive(req.keep_alive());
  return res;
}

template <typename T>
void write_response(tcp::socket &socket, http::response<T> &&msg,
                    beast::error_code ec) {

  beast::http::write(socket, msg, ec);
}

void handle_session(tcp::socket &socket) {
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

    auto common_response = handle_request(std::move(req));

    if (common_response.index() == 0) {
      write_response(socket, std::move(std::get<0>(common_response)), ec);
    } else if (common_response.index() == 1) {
      write_response(socket, std::move(std::get<1>(common_response)), ec);
    }

    if (ec) {
      return fail(ec, "write");
    }
  }

  socket.shutdown(tcp::socket::shutdown_send, ec);
}

int main(int argc, char *argv[]) {
  try {
    if (argc != 4) {
      std::cerr << "Usage: server <address> <server_port> <concurrency_level>\n"
                << "Example:\n"
                << "    server 127.0.0.1 8080 1\n";
      return EXIT_FAILURE;
    }
    auto const address = net::ip::make_address(argv[1]);
    auto const port = static_cast<unsigned short>(std::atoi(argv[2]));
    auto const concurrency_level =
        static_cast<unsigned int>(std::atoi(argv[3]));

    net::thread_pool pool{concurrency_level};

    net::io_context ioc{1};

    tcp::acceptor acceptor{ioc, {address, port}};
    for (;;) {
      tcp::socket socket{ioc};

      acceptor.accept(socket);

      // Single-threaded version of program should be following:
      // handle_session(std::move(socket));
      net::post(pool, std::bind(&handle_session, std::move(socket)));
    }
  } catch (const std::exception &e) {
    std::cerr << "Error: " << e.what() << std::endl;
    return EXIT_FAILURE;
  }
}