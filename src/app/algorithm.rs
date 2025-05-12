use cxx::{CxxVector, UniquePtr};

#[cxx::bridge]
mod ffi {
    #[derive(Clone)]
    struct User {
        id: u64,
        stamps: CxxVector<u64>,
    }

    #[derive(Clone)]
    struct Spare {
        stamp: u64,
        day: u64,
    }

    unsafe extern "C++" {
        include!("distribute.h");

        fn distribute(
            users: &CxxVector<User>,
            spares: &CxxVector<Spare>,
        ) -> UniquePtr<CxxVector<User>>;
    }
}

pub fn distribute(
    users: Vec<User>,
    spares: Vec<Spare>,
) -> Vec<User> {
    let mut cv_users = CxxVector::<ffi::User>::new();
    for u in users {
        let mut cv_stamps = CxxVector::<u64>::new();
        for s in u.stamps {
            cv_stamps.push(s);
        }
        cv_users.push( ffi::User { id: u.id, stamps: cv_stamps } );
    }

    let mut cv_spares = CxxVector::<ffi::Spare>::new();
    for s in spares {
        cv_spares.push( ffi::Spare { stamp: s.stamp, day: s.day } );
    }

    let uptr: UniquePtr<CxxVector<ffi::User>> = ffi::distribute(&cv_users, &cv_spares);
    let cv_res = uptr.as_ref().expect("ffi::distribute returned null");

    let mut out = Vec::with_capacity(cv_res.len());
    for ru in cv_res.iter() {
        let mut stamps = Vec::with_capacity(ru.stamps.len());
        for &st in ru.stamps.iter() {
            stamps.push(st);
        }
        out.push(User { id: ru.id, stamps });
    }
    out
}
