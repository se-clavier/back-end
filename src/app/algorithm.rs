use std::cmp::min;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Edge {
    to: usize,
    capacity: i32,
    flow: i32,
    rev: usize,
}

#[derive(Debug, Clone)]
struct Dinic {
    graph: Vec<Vec<Edge>>,
    n: usize,
}

impl Dinic {
    fn new(n: usize) -> Self {
        Dinic {
            graph: vec![Vec::new(); n],
            n,
        }
    }

    fn add_edge(&mut self, from: usize, to: usize, capacity: i32) {
        let to_len = self.graph[to].len();
        let from_len = self.graph[from].len();
        self.graph[from].push(Edge {
            to,
            capacity,
            flow: 0,
            rev: to_len,
        });
        self.graph[to].push(Edge {
            to: from,
            capacity: 0,
            flow: 0,
            rev: from_len,
        });
    }

    fn bfs(&self, s: usize, t: usize, level: &mut [i32]) -> bool {
        level.fill(-1);
        level[s] = 0;
        let mut queue = VecDeque::new();
        queue.push_back(s);

        while let Some(u) = queue.pop_front() {
            for edge in &self.graph[u] {
                if edge.capacity - edge.flow > 0 && level[edge.to] == -1 {
                    level[edge.to] = level[u] + 1;
                    queue.push_back(edge.to);
                }
            }
        }
        level[t] != -1
    }

    fn dfs(
        &mut self,
        u: usize,
        t: usize,
        level: &Vec<i32>,
        flow: i32,
        start: &mut Vec<usize>,
    ) -> i32 {
        if u == t {
            return flow;
        }
        while start[u] < self.graph[u].len() {
            let i = start[u];
            let (capacity, to, rev) = {
                let edge = &self.graph[u][i];
                (edge.capacity - edge.flow, edge.to, edge.rev)
            };

            if capacity > 0 && level[to] == level[u] + 1 {
                let pushed = self.dfs(to, t, level, min(flow, capacity), start);
                if pushed > 0 {
                    self.graph[u][i].flow += pushed;
                    self.graph[to][rev].flow -= pushed;
                    return pushed;
                }
            }
            start[u] += 1;
        }
        0
    }

    fn max_flow(&mut self, s: usize, t: usize) -> i32 {
        let mut total_flow = 0;
        let mut level = vec![0; self.n];
        while self.bfs(s, t, &mut level) {
            let mut start = vec![0; self.n];
            while let Some(flow) = Some(self.dfs(s, t, &level, i32::MAX, &mut start)) {
                if flow == 0 {
                    break;
                }
                total_flow += flow;
            }
        }
        total_flow
    }
}

pub fn max_flow(users: Vec<(i64, Vec<usize>)>, spares: Vec<usize>) -> Vec<Option<i64>> {
    let mut dinic = Dinic::new(users.len() * 7 + spares.len() + 2);
    let s = users.len() * 7 + spares.len();
    let t = users.len() * 7 + spares.len() + 1;
    let mut user_nodes = Vec::new();
    for (user_id, stamps) in users {
        let n = spares.len() + user_nodes.len();
        for day in 0..7 {
            user_nodes.push(user_id);
            dinic.add_edge(n + day, t, 1);
        }
        for stamp in stamps {
            let user_node = n + spares[stamp];
            dinic.add_edge(stamp, user_node, 1);
        }
    }
    for stamp in 0..spares.len() {
        dinic.add_edge(s, stamp, 1);
    }
    dinic.max_flow(s, t);
    (0..spares.len())
        .map(|stamp| {
            for edge in dinic.graph[stamp].iter() {
                if edge.flow > 0 {
                    return Some(user_nodes[edge.to - spares.len()]);
                }
            }
            None
        })
        .collect()
}
