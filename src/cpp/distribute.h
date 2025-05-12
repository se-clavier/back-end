#include <iostream>
#include <vector>
#include "mcmf.h"
#include <cstdint>
#include <memory>
using std::vector;
using std::byte;
using std::runtime_error;


struct User {
	uint64_t id;
	vector<uint64_t> stamps;
};
struct Spare {
	uint64_t stamp, day;
};

struct distribution {
	vector<User> user;
	vector<Spare> spare;
	void init(const vector<User> &users, const vector<Spare> &spares, int64_t spare_size) {
		if (spare_size != (int64_t) spares.size()) {
			throw runtime_error("spare_size != spares.size()");
		}
		user.clear();
		spare.clear();
		user.shrink_to_fit();
		spare.shrink_to_fit();
		user.resize(users.size());
		spare.resize(spare_size);
		for (size_t i = 0; i < (size_t) spare_size; i++) {
			spare[i] = spares[i];
		}
		for (size_t i = 0; i < users.size(); i++) {
			user[i] = users[i];
		}
	}
	vector<User> solve() {
		mcmf::init();
		int32_t s = 1;
		int32_t n = user.size() * 8 + spare.size() + 2;
		int32_t t = n;
		mcmf::s = s;
		mcmf::t = t;
		mcmf::n = n;
		mcmf::set_n(n);
		for (size_t i = 0; i < user.size(); i++) {
			mcmf::addEdge(s, i + 2, 0, 1, 20);
			mcmf::addEdge(s, i + 2, 0, 1, 50);
			mcmf::addEdge(s, i + 2, 0, 1, 100);
		}
		for (size_t i = 1; i <= 7; ++i) {
			for (size_t j = 0; j < user.size(); j++) {
				mcmf::addEdge(j + 2, j + 2 + i * user.size(), 0, 1, 0);
			}
		}
		for (size_t i = 0; i < user.size(); ++i) {
			for (size_t j = 0; j < user[i].stamps.size(); ++j) {
				mcmf::addEdge(i + 2 + (spare[user[i].stamps[j]].day + 1) * user.size(), n - spare.size() + spare[user[i].stamps[j]].stamp, 0, 1, 0); 
			}
		}
		for (size_t i = 0; i < spare.size(); ++i) {
			mcmf::addEdge(n - spare.size() + i, t, 0, 1, 0);
		}
		mcmf::solve();
		int64_t a0 = 1 + user.size();
		int64_t a1 = 1 + user.size() * 8;
		int64_t a2 = n - 1;
		vector<User> res(user.size());
		for (size_t i = 2; i < mcmf::e.size(); ++i) {
			if (mcmf::e[i].u <= a1 && mcmf::e[i].u > a0 && mcmf::e[i].v <= a2 && mcmf::e[i].v > a1) {
				res[mcmf::e[i].u - a0 - 1].stamps.push_back(mcmf::e[i].v - a2 - 1);
			}
		}
		for (size_t i = 0; i < user.size(); ++i) {
			res[i].id = user[i].id;
			std::sort(res[i].stamps.begin(), res[i].stamps.end());
		}
		return res;
	}
};


std::unique_ptr<std::vector<User>> distribute(const vector<User> &users, const vector<Spare> &spares) {
	distribution sol;
	sol.init(users, spares, spares.size());
	vector<User> method = sol.solve();
	return std::make_unique<std::vector<User>>(std::move(method));

}
