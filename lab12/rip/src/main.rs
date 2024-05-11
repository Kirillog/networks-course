use socket2::SockAddr;

use std::collections::VecDeque;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::slice::Iter;
use std::sync::Arc;
use std::thread::scope;
use std::time::Duration;

use rand::distributions::Distribution;
use rand::rngs::ThreadRng;
use rand::{distributions::Uniform, thread_rng, Rng};

use rand::seq::SliceRandom;

const DEFAULT_ROUTERAMOUNT: usize = 5;
const DEFAULT_SIMULATIONCYCLES: usize = 10;

type RouterId = usize;

#[derive(Clone, Copy, Debug)]
struct Route {
    dest_ip: Ipv4Addr,
    metric: u32,
}

impl Route {
    fn new() -> Self {
        Route {
            dest_ip: Ipv4Addr::new(0, 0, 0, 0),
            metric: 16,
        }
    }
}

struct Router {
    id: RouterId,
    ip: Ipv4Addr,
    adj_list: Vec<RouterId>,
    table: Vec<Route>,
}

impl Router {
    fn update(
        &mut self,
        Message {
            id: from_id,
            ip: from_ip,
            table: from_table,
        }: Message,
    ) {
        // println!("Recv from {} at {}", from_id, self.id);
        let mut changed = false;
        if self.table[from_id].dest_ip.is_unspecified() {
            changed = true;
            self.table[from_id].dest_ip = from_ip;
            self.table[from_id].metric = 1;
        }
        for (id, route) in from_table.iter().enumerate() {
            if route.dest_ip.is_unspecified() {
                continue;
            }
            if self.table[id].dest_ip.is_unspecified() {
                changed = true;
                self.table[id] = route.clone();
                self.table[id].metric += 1;
            } else if self.table[id].metric > route.metric + 1 {
                changed = true;
                self.table[id].metric = route.metric + 1;
            }
        }
        if changed {
            println!("Router {}:\n{:?}", self.ip, self.table);
        }
    }
}

struct Message {
    ip: Ipv4Addr,
    id: RouterId,
    table: Vec<Route>,
}

impl Message {
    fn as_bytes(&self) -> Vec<u8> {
        self.table
            .iter()
            .map(|route| {
                route
                    .dest_ip
                    .octets()
                    .iter()
                    .chain(route.metric.to_le_bytes().iter())
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .chain(self.ip.octets().iter().cloned())
            .chain(self.id.to_le_bytes().iter().cloned())
            .collect::<Vec<_>>()
    }

    fn from_bytes(bytes: &[u8], table_size: usize) -> Message {
        let mut bytes_iter = bytes.iter();

        let get_4bytes = |bytes_iter: &mut Iter<u8>| {
            bytes_iter
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()
        };

        let mut table = Vec::new();
        for _ in 0..table_size {
            let dest_ip = Ipv4Addr::from(get_4bytes(bytes_iter.by_ref()));
            let metric = u32::from_le_bytes(get_4bytes(bytes_iter.by_ref()));
            table.push(Route { dest_ip, metric });
        }

        let ip = Ipv4Addr::from(get_4bytes(bytes_iter.by_ref()));

        let id = usize::from_le_bytes(
            bytes_iter
                .take((usize::BITS / 8) as usize)
                .cloned()
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );

        Message { table, ip, id }
    }
}

struct Network {
    routers: Vec<Router>,
}

impl Network {
    fn gen_random(router_amount: usize) -> Self {
        let mut rng = thread_rng();

        let gen_random_ip =
            |rng: &mut ThreadRng| Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen());

        let mut routers = Vec::<Router>::new();
        let mut generated_ids = Vec::<usize>::new();

        for router_id in 0..router_amount {
            let ip = gen_random_ip(&mut rng);
            let adj_list = if router_id > 0 {
                let dist = Uniform::new(1, router_id + 1);
                generated_ids.shuffle(&mut rng);
                let adj_list = generated_ids[..dist.sample(&mut rng)].to_vec();
                for adj in &adj_list {
                    routers[*adj].adj_list.push(router_id);
                }
                adj_list
            } else {
                Vec::new()
            };
            let mut new_router = Router {
                id: router_id,
                ip,
                adj_list,
                table: vec![Route::new(); router_amount],
            };
            new_router.table[router_id].metric = 0;
            new_router.table[router_id].dest_ip = ip;
            routers.push(new_router);
            generated_ids.push(router_id);
        }

        Network { routers }
    }
}

fn main() -> io::Result<()> {
    let Network { routers } = Network::gen_random(DEFAULT_ROUTERAMOUNT);
    let mut routers = VecDeque::from(routers);
    let addrs = gen_addrs();

    let global_sockets = gen_sockets(&addrs);

    scope(|s| {
        for _ in 0..DEFAULT_ROUTERAMOUNT {
            let copy_addrs: Vec<SocketAddrV4> = addrs.clone();
            let local_sockets = global_sockets.clone();
            let mut router = routers.pop_front().unwrap();
            s.spawn(move || {
                let sock = local_sockets[router.id].clone();

                for _ in 0..DEFAULT_SIMULATIONCYCLES {
                    for adj_id in &router.adj_list {
                        let adj_addr = copy_addrs[*adj_id];
                        let data = Message {
                            id: router.id,
                            ip: router.ip,
                            table: router.table.clone(),
                        };
                        local_sockets[*adj_id]
                            .send_to(data.as_bytes().as_slice(), adj_addr)
                            .unwrap();
                    }
                    let mut buf = [0u8; 25000];
                    loop {
                        match sock.recv_from(&mut buf) {
                            Ok((size, _)) => {
                                router.update(Message::from_bytes(
                                    &buf[0..size],
                                    DEFAULT_ROUTERAMOUNT,
                                ));
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }

                println!(
                    "Final state of router {} table:\n {:?}",
                    router.ip, router.table
                );
            });
        }
    });

    Ok(())
}

fn gen_addrs() -> Vec<SocketAddrV4> {
    (0..DEFAULT_ROUTERAMOUNT)
        .into_iter()
        .map(|_| {
            let port = portpicker::pick_unused_port().unwrap();
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)
        })
        .collect::<Vec<_>>()
}

fn gen_sockets(addrs: &Vec<SocketAddrV4>) -> Vec<Arc<std::net::UdpSocket>> {
    (0..DEFAULT_ROUTERAMOUNT)
        .into_iter()
        .map(|id| {
            let sock = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;

            sock.set_reuse_address(true)?;
            sock.set_read_timeout(Some(Duration::from_millis(100)))?;
            sock.bind(&SockAddr::from(addrs[id]))?;

            let std_sock = std::net::UdpSocket::from(sock);
            Ok(Arc::new(std_sock))
        })
        .collect::<io::Result<Vec<_>>>()
        .unwrap()
}
