#pragma once

#include <iostream>
#include <cstring>
#include <algorithm>
#include <vector>
#include <queue>


using std::min;
using std::vector;
using std::queue;

namespace mcmf {
    const int INF = 0x3f3f3f3f;
	int n, m, s, t, maxflow, cost;
	int in, S, T;
	int a0, a1;
	struct E {
		int u, next, v, w, c;
	};
	vector<int> d, incf, pre, head, a, vis;
	vector<E> e;
	queue<int> q;
    void inline add(int u, int v, int w, int c) {
		e.push_back({ u, head[u], v, w, c });
		head[u] = e.size() - 1;
    }
    void inline addE(int u, int v, int w, int c) {
        add(u, v, w, c);
		add(v, u, 0, -c);
    }
    bool spfa() {
		std::fill(vis.begin(), vis.end(), 0);
		std::fill(d.begin(), d.end(), INF);
		q.push(S);
        d[S] = 0; incf[S] = 2e9;
		while (q.size()) {
			int u = q.front(); q.pop(); vis[u] = 0;
            for (int i = head[u]; i; i = e[i].next) {
                int v = e[i].v;
                if (e[i].w && d[u] + e[i].c < d[v]) {
                    d[v] = d[u] + e[i].c;
                    pre[v] = i;
                    incf[v] = min(incf[u], e[i].w);
                    if (!vis[v]) {
						q.push(v);
                        vis[v] = 1;
                    }
                }
            }
        } 
        return d[T] != INF;
    }
    void update() {
        int x = T;
        while (x != S) {
            int i = pre[x];
            e[i].w -= incf[T], e[i ^ 1].w += incf[T];
            x = e[i ^ 1].v;
        }
        maxflow += incf[T];
        cost += d[T] * incf[T];
    }

    void inline addEdge(int u, int v, int l, int d, int c) {
        a[v] += l, a[u] -= l;
        addE(u, v, d - l, c);
    }

    void inline work() {
        while (spfa()) update();
    }

    void inline ADD(int u, int v, int w, int c) {
        if (c >= 0) addEdge(u, v, 0, w, c); 
        else a[v] += w, a[u] -= w, addEdge(v, u, 0, w, -c), a1 += c * w;
    }

    void inline solve() {
		S = n + 1;
		T = n + 2;
        for (int i = 1; i <= n; i++) {
            if (!a[i]) continue;
            if (a[i] > 0) addEdge(S, i, 0, a[i], 0);
            else addEdge(i, T, 0, -a[i], 0);
        }
        addEdge(T, S, 0, INF, 0);
        work();
        S = s, T = t;
        a1 += cost;
        maxflow = cost = 0;
		e[e.size() - 1].w = 0;
		e[e.size() - 2].w = 0;
        work();
        a0 += maxflow, a1 += cost;
    }
	void init() {
		n = 0;
		m = 0;
		s = 0;
		t = 0;
		maxflow = 0;
		cost = 0;
		in = 0;
		S = 0;
		T = 0;
		a0 = 0;
		a1 = 0;
		d.clear();
		d.shrink_to_fit();
		incf.clear();
		incf.shrink_to_fit();
		pre.clear();
		pre.shrink_to_fit();
		head.clear();
		head.shrink_to_fit();
		a.clear();
		a.shrink_to_fit();
		vis.clear();
		vis.shrink_to_fit();
		e.clear();
		e.shrink_to_fit();
		while (q.size()) q.pop();
		e.push_back({0, 0, 0, 0, 0});
		e.push_back({0, 0, 0, 0, 0});
	}
	void set_n(int32_t n) {
		d.resize(n + 3, 0);
		incf.resize(n + 3, 0);
		pre.resize(n + 3, 0);
		head.resize(n + 3, 0);
		a.resize(n + 3, 0);
		vis.resize(n + 3, 0);
	}
}
