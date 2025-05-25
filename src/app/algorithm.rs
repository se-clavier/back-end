use std::cmp::min;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Edge {
    pub u: i32,
    pub next: usize,
    pub v: i32,
    pub w: i32,
    pub c: i32,
}

pub struct Mcmf {
    pub n: i32,
    pub m: i32,
    pub s: i32,
    pub t: i32,
    pub maxflow: i32,
    pub cost: i32,
    pub in_field: i32,
    pub ss: i32,
    pub tt: i32,
    pub a0: i32,
    pub a1: i32,
    pub d: Vec<i32>,
    pub incf: Vec<i32>,
    pub pre: Vec<usize>,
    pub head: Vec<usize>,
    pub a: Vec<i32>,
    pub vis: Vec<bool>,
    pub e: Vec<Edge>,
    pub q: VecDeque<i32>,
}

impl Mcmf {
    pub const INF: i32 = 0x3f3f3f3f;

    pub fn new() -> Self {
        let mut m = Mcmf {
            n: 0,
            m: 0,
            s: 0,
            t: 0,
            maxflow: 0,
            cost: 0,
            in_field: 0,
            ss: 0,
            tt: 0,
            a0: 0,
            a1: 0,
            d: Vec::new(),
            incf: Vec::new(),
            pre: Vec::new(),
            head: Vec::new(),
            a: Vec::new(),
            vis: Vec::new(),
            e: Vec::new(),
            q: VecDeque::new(),
        };
        m.e.push(Edge {
            u: 0,
            next: 0,
            v: 0,
            w: 0,
            c: 0,
        });
        m.e.push(Edge {
            u: 0,
            next: 0,
            v: 0,
            w: 0,
            c: 0,
        });
        m
    }

    pub fn init(&mut self) {
        self.n = 0;
        self.m = 0;
        self.s = 0;
        self.t = 0;
        self.maxflow = 0;
        self.cost = 0;
        self.in_field = 0;
        self.ss = 0;
        self.tt = 0;
        self.a0 = 0;
        self.a1 = 0;
        self.d.clear();
        self.incf.clear();
        self.pre.clear();
        self.head.clear();
        self.a.clear();
        self.vis.clear();
        self.e.truncate(2);
        self.q.clear();
    }

    pub fn set_n(&mut self, n: i32) {
        let sz = (n + 3) as usize;
        self.d.resize(sz, 0);
        self.incf.resize(sz, 0);
        self.pre.resize(sz, 0);
        self.head.resize(sz, 0);
        self.a.resize(sz, 0);
        self.vis.resize(sz, false);
    }

    fn add(&mut self, u: i32, v: i32, w: i32, c: i32) {
        let idx = self.e.len();
        self.e.push(Edge {
            u,
            next: self.head[u as usize],
            v,
            w,
            c,
        });
        self.head[u as usize] = idx;
    }

    fn add_e(&mut self, u: i32, v: i32, w: i32, c: i32) {
        self.add(u, v, w, c);
        self.add(v, u, 0, -c);
    }

    pub fn add_edge(&mut self, u: i32, v: i32, l: i32, d: i32, c: i32) {
        self.a[v as usize] += l;
        self.a[u as usize] -= l;
        self.add_e(u, v, d - l, c);
    }

    pub fn add_signed(&mut self, u: i32, v: i32, w: i32, c: i32) {
        if c >= 0 {
            self.add_edge(u, v, 0, w, c);
        } else {
            self.a[v as usize] += w;
            self.a[u as usize] -= w;
            self.add_edge(v, u, 0, w, -c);
            self.a1 += c * w;
        }
    }

    fn spfa(&mut self) -> bool {
        self.q.clear();
        self.vis.fill(false);
        self.d.fill(Self::INF);
        self.q.push_back(self.ss);
        self.d[self.ss as usize] = 0;
        self.incf[self.ss as usize] = std::i32::MAX;
        while let Some(u) = self.q.pop_front() {
            self.vis[u as usize] = false;
            let mut i = self.head[u as usize];
            while i != 0 {
                let e = &self.e[i];
                if e.w > 0 && self.d[u as usize] + e.c < self.d[e.v as usize] {
                    self.d[e.v as usize] = self.d[u as usize] + e.c;
                    self.pre[e.v as usize] = i;
                    self.incf[e.v as usize] = min(self.incf[u as usize], e.w);
                    if !self.vis[e.v as usize] {
                        self.vis[e.v as usize] = true;
                        self.q.push_back(e.v);
                    }
                }
                i = e.next;
            }
        }
        self.d[self.tt as usize] != Self::INF
    }

    /// 沿增广路更新流与费用
    fn update(&mut self) {
        let mut x = self.tt;
        let flow = self.incf[self.tt as usize];
        while x != self.ss {
            let i = self.pre[x as usize];
            self.e[i].w -= flow;
            let ri = i ^ 1;
            self.e[ri].w += flow;
            x = self.e[ri].v;
        }
        self.maxflow += flow;
        self.cost += self.d[self.tt as usize] * flow;
    }

    /// 主循环：不断 SPFA + 更新
    pub fn work(&mut self) {
        while self.spfa() {
            self.update();
        }
    }

    /// 求解
    pub fn solve(&mut self) {
        // 构造超级源汇
        self.ss = self.n + 1;
        self.tt = self.n + 2;
        for i in 1..=self.n {
            let ai = self.a[i as usize];
            if ai > 0 {
                self.add_edge(self.ss, i, 0, ai, 0);
            } else if ai < 0 {
                self.add_edge(i, self.tt, 0, -ai, 0);
            }
        }
        // 保证原图中循环流可行
        self.add_edge(self.tt, self.ss, 0, Self::INF, 0);
        self.work();

        // 切换为真正的源汇
        self.ss = self.s;
        self.tt = self.t;
        self.a1 += self.cost;
        self.maxflow = 0;
        self.cost = 0;
        // 将最后两条边容量置零
        let len = self.e.len();
        self.e[len - 1].w = 0;
        self.e[len - 2].w = 0;
        self.work();
        self.a0 += self.maxflow;
        self.a1 += self.cost;
    }
}

use std::error::Error;

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub stamps: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct Spare {
    pub stamp: u64,
    pub day: u64,
}

pub struct Distribution {
    user: Vec<User>,
    spare: Vec<Spare>,
    mf: Mcmf, // 最大流最小费用算法
}

impl Distribution {
    pub fn new() -> Self {
        Distribution {
            user: Vec::new(),
            spare: Vec::new(),
            mf: Mcmf::new(),
        }
    }

    pub fn init(
        &mut self,
        users: &[User],
        spares: &[Spare],
        spare_size: usize,
    ) -> Result<(), Box<dyn Error>> {
        if spare_size != spares.len() {
            return Err("spare_size != spares.len()".into());
        }
        self.user.clear();
        self.spare.clear();
        self.user.reserve(users.len());
        self.spare.reserve(spare_size);
        self.user.extend_from_slice(users);
        self.spare.extend_from_slice(spares);
        Ok(())
    }

    pub fn solve(&mut self) -> Vec<User> {
        self.mf.init();
        let s = 1;
        let n_nodes = (self.user.len() * 8) as i32 + self.spare.len() as i32 + 2;
        let t = n_nodes;
        self.mf.n = n_nodes;
        self.mf.s = s;
        self.mf.t = t;
        self.mf.set_n(n_nodes);

        for i in 0..self.user.len() {
            let u = (i + 2) as i32;
            self.mf.add_edge(s, u, 0, 1, 20);
            self.mf.add_edge(s, u, 0, 1, 50);
            self.mf.add_edge(s, u, 0, 1, 100);
        }
        for day in 1..=7 {
            for j in 0..self.user.len() {
                let from = (j + 2) as i32;
                let to = (j + 2 + day * self.user.len()) as i32;
                self.mf.add_edge(from, to, 0, 1, 0);
            }
        }
        for (i, user) in self.user.iter().enumerate() {
            for &stamp in &user.stamps {
                let from =
                    (i as u64 + 2 + (self.spare[stamp as usize].day + 1) * self.user.len() as u64)
                        as i32;
                let to =
                    n_nodes - self.spare.len() as i32 + self.spare[stamp as usize].stamp as i32;
                self.mf.add_edge(from, to, 0, 1, 0);
            }
        }
        for i in 0..self.spare.len() {
            let from = n_nodes - self.spare.len() as i32 + i as i32;
            self.mf.add_edge(from, t, 0, 1, 0);
        }
        self.mf.solve();
        let a0 = 1 + self.user.len() as i64;
        let a1 = 1 + (self.user.len() * 8) as i64;
        let a2 = (self.mf.n - 1) as i64;
        let mut res = vec![
            User {
                id: 0,
                stamps: Vec::new()
            };
            self.user.len()
        ];
        for e in &self.mf.e {
            let u = e.u as i64;
            let v = e.v as i64;
            if u > a0 && u <= a1 && v > a1 && v <= a2 && e.w == 0 {
                let idx = ((u - a0 - 1) % self.user.len() as i64) as usize;
                let stamp = (v - a1 - 1) as u64;
                res[idx].stamps.push(stamp);
            }
        }
        for (i, user) in self.user.iter().enumerate() {
            res[i].id = user.id;
            res[i].stamps.sort_unstable();
        }
        res
    }
}

// 分配琴房到用户空闲时间
fn distribute(users: Vec<User>, spares: Vec<Spare>) -> Vec<User> {
    let mut sol = Distribution::new();
    sol.init(&users, &spares, spares.len())
        .expect("spare_size must equal spares.len()");
    sol.solve()
}

pub fn max_flow(
    entries: Vec<(i64, Vec<usize>)>,
    spares: Vec<usize>,
) -> Vec<Option<i64>> {
    let users: Vec<User> = entries
        .into_iter()
        .map(|(id, stamps)| User {
            id: id as u64,
            stamps: stamps.into_iter().map(|s| s as u64).collect(),
        })
        .collect();

    let sp_len = spares.len();

    let spare_structs: Vec<Spare> = spares
        .into_iter()
        .enumerate()
        .map(|(idx, day)| Spare {
            stamp: idx as u64,
            day: day as u64,
        })
        .collect();

    let assigned = distribute(users, spare_structs);

    let mut res = vec![None; sp_len];
    for user in assigned {
        let uid = user.id as i64;
        for stamp in user.stamps {
            let idx = stamp as usize;
            if idx < sp_len {
                res[idx] = Some(uid);
            }
        }
    }
    res
}


#[cfg(test)]
mod tests {
    use super::*;

	use rand::{rng, seq::SliceRandom};
    #[test]
    fn test_mcmf_sample1() {
        let mut mf = Mcmf::new();
        mf.n = 4;
        mf.s = 4;
        mf.t = 3;
        mf.set_n(4);

        mf.add_signed(4, 2, 30, 2);
        mf.add_signed(4, 3, 20, 3);
        mf.add_signed(2, 3, 20, 1);
        mf.add_signed(2, 1, 30, 9);
        mf.add_signed(1, 3, 40, 5);

        mf.solve();

        assert_eq!(mf.a0, 50, "期望最大流 a0 = 50，但实际是 {}", mf.a0);
        assert_eq!(mf.a1, 280, "期望最小费用 a1 = 280，但实际是 {}", mf.a1);
    }
    #[test]
    fn test_mcmf_sample2() {
        let mut mf = Mcmf::new();
        mf.n = 5;
        mf.s = 1;
        mf.t = 5;
        mf.set_n(5);

        mf.add_signed(1, 3, 2, 4);
        mf.add_signed(1, 2, 2, 3);
        mf.add_signed(3, 5, 2, 2);
        mf.add_signed(3, 2, 1, -1);
        mf.add_signed(2, 4, 2, -2);
        mf.add_signed(4, 3, 1, -1);
        mf.add_signed(4, 5, 1, 3);

        mf.solve();

        assert_eq!(mf.a0, 3, "期望最大流 a0 = 3，但实际是 {}", mf.a0);
        assert_eq!(mf.a1, 12, "期望最小费用 a1 = 12，但实际是 {}", mf.a1);
    }
    // empty_test
    #[test]
    fn test_distribution_sample1() {
        let users = vec![User {
            id: 0,
            stamps: vec![],
        }];
        let spares = vec![Spare { day: 0, stamp: 0 }];
        let res = distribute(users, spares);
        println!("res: {:?}", res);
        assert_eq!(res.len(), 1, "期望结果长度为 1，但实际是 {}", res.len());
    }
    #[test]
    fn test_distribution_sample2() {
        let users = vec![User {
            id: 0,
            stamps: vec![0],
        }];
        let spares = vec![Spare { day: 0, stamp: 0 }];
        let res = distribute(users, spares);
        // println!("res: {:?}", res);
        assert_eq!(res.len(), 1, "期望结果长度为 1，但实际是 {}", res.len());
    }

    #[test]
    fn test_distribution_sample3() {
        let users = vec![User {
            id: 0,
            stamps: vec![0, 1],
        }];
        let spares = vec![Spare { day: 0, stamp: 0 }, Spare { day: 0, stamp: 1 }];
        let res = distribute(users, spares);
        // println!("res: {:?}", res);
        assert_eq!(res.len(), 1, "期望结果长度为 1，但实际是 {}", res.len());
    }

    #[test]
    fn test_distribution_sample4() {
        let mut spares = Vec::new();
        for day in 0..7 {
            for slot in 0..2 {
                spares.push(Spare {
                    stamp: (day * 2 + slot) as u64,
                    day: day as u64,
                });
            }
        }

        let mut rng = rng();
        let all_slots: Vec<u64> = (0..14).collect();
        let users: Vec<User> = (0..10)
            .map(|i| {
                let mut picks = all_slots.clone();
                picks.shuffle(&mut rng);
                picks.truncate(5);
                User { id: i as u64, stamps: picks }
            })
            .collect();

		println!("初始");
		for user in &users {
            println!("  用户 {}: 时隙 {:?}", user.id, user.stamps);
        }
        let result = distribute(users.clone(), spares.clone());

        println!("分配结果：");
        for user in &result {
            println!("  用户 {}: 时隙 {:?}", user.id, user.stamps);
        }

        assert_eq!(result.len(), users.len(), "结果用户数量应与输入一致");
        for user in result {
            assert!(user.stamps.len() <= 5, "用户分配时隙不能超过 5");
            for &stamp in &user.stamps {
                assert!(stamp < 14, "时隙索引应在 0..14 范围内");
            }
        }
    }
}