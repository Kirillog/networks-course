use std::{
    cmp::min,
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
    usize::MAX,
};

type NodeId = usize;

struct DistVec {
    node_id: NodeId,
    dist: Vec<usize>,
}

struct Edge<'a> {
    to: NodeId,
    dist: &'a AtomicUsize,
}

impl<'a> Edge<'a> {
    fn new(to: NodeId, dist: &'a AtomicUsize) -> Self {
        Edge { to, dist }
    }
}

#[derive(PartialEq)]
struct LocalEdge {
    to: NodeId,
    dist: usize,
}

fn get_local_copy(adj_list: &Vec<Edge>) -> Vec<LocalEdge> {
    adj_list
        .iter()
        .map(|edge| LocalEdge {
            to: edge.to,
            dist: edge.dist.load(Ordering::Acquire),
        })
        .collect()
}

fn take_snapshot(dist_snapshots: &Vec<Arc<Mutex<Vec<Vec<usize>>>>>) {
    std::thread::sleep(Duration::from_millis(100)); // Wait some time until system establishes
    println!("Dump distances:");
    for (node_id, snapshot) in dist_snapshots.iter().enumerate() {
        let dist = snapshot.lock().unwrap();
        println!("{} node:\n{:?}", node_id, dist[node_id]);
    }
}

fn main() {
    let node_count = 4;
    let distances = vec![
        AtomicUsize::new(1),
        AtomicUsize::new(1),
        AtomicUsize::new(3),
        AtomicUsize::new(2),
        AtomicUsize::new(7),
    ];
    let mut graph: VecDeque<Vec<Edge>> = VecDeque::from([
        vec![
            Edge::new(1, &distances[0]),
            Edge::new(2, &distances[2]),
            Edge::new(3, &distances[4]),
        ],
        vec![Edge::new(0, &distances[0]), Edge::new(2, &distances[1])],
        vec![
            Edge::new(0, &distances[2]),
            Edge::new(1, &distances[1]),
            Edge::new(3, &distances[3]),
        ],
        vec![Edge::new(0, &distances[4]), Edge::new(2, &distances[3])],
    ]);

    let mut send_sides = Vec::new();
    let mut recv_sides = VecDeque::new();

    for _ in 0..node_count {
        let (send_side, recv_side) = std::sync::mpsc::channel::<DistVec>();
        send_sides.push(send_side);
        recv_sides.push_back(recv_side);
    }

    let dist_snapshots: Vec<_> = (0..node_count)
        .into_iter()
        .map(|_| Arc::new(Mutex::new(vec![vec![MAX; node_count]; node_count])))
        .collect();

    thread::scope(|s| {
        for node_id in 0..node_count {
            let dists_shared = dist_snapshots[node_id].clone();
            let adj_list = graph.pop_front().unwrap();
            let recv_side = recv_sides.pop_front().unwrap();
            let send_sides = send_sides.clone();
            s.spawn(move || {
                let mut local_adj_list = get_local_copy(&adj_list);

                let send_other_nodes =
                    |local_adj_list: &Vec<LocalEdge>, dist: &Vec<usize>, id: NodeId| {
                        for edge in local_adj_list {
                            send_sides[edge.to]
                                .send(DistVec {
                                    node_id: id,
                                    dist: dist.clone(),
                                })
                                .unwrap();
                        }
                    };

                let update_adj_dists = |local_adj_list: &Vec<LocalEdge>| {
                    let mut dists = dists_shared.lock().unwrap();
                    dists[node_id][node_id] = 0;
                    for edge in local_adj_list {
                        dists[node_id][edge.to] = edge.dist;
                    }
                    send_other_nodes(local_adj_list, &dists[node_id], node_id);
                };

                update_adj_dists(&local_adj_list);

                loop {
                    {
                        let mut dists = dists_shared.lock().unwrap();
                        for DistVec {
                            node_id: from_id,
                            dist: vec_dist,
                        } in recv_side.try_iter()
                        {
                            // println!(
                            //     "Get for {} from {}, {:?}",
                            //     node_id, from_id, vec_dist
                            // );
                            let old_vec = dists[node_id].clone();
                            dists[from_id] = vec_dist;
                            dists[node_id] = vec![MAX; node_count];
                            dists[node_id][node_id] = 0;
                            for y in 0..node_count {
                                for edge in &local_adj_list {
                                    dists[node_id][y] = min(
                                        dists[node_id][y],
                                        edge.dist.saturating_add(dists[edge.to][y]),
                                    );
                                }
                            }
                            if old_vec != dists[node_id] {
                                send_other_nodes(&local_adj_list, &dists[node_id], node_id);
                            }
                        }
                    }
                    let new_copy = get_local_copy(&adj_list);
                    if local_adj_list == new_copy {
                        std::thread::sleep(Duration::from_millis(1)); // Avoid busy waiting (spinlock)
                    } else {
                        local_adj_list = new_copy;
                        update_adj_dists(&local_adj_list);
                    }
                }
            });
        }
        take_snapshot(&dist_snapshots);
        distances[0].store(100, Ordering::Release);
        take_snapshot(&dist_snapshots);
    });
}
