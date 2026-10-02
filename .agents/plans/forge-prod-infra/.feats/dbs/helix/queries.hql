QUERY GetBulletSkills (bullet_id: ID) =>
    skills <- N<Skill>(bullet_id)::Out<Has>
    RETURN following

QUERY GetJDSkills (jd_id: ID) =>
    reqs <- N<Req>(jd_id)::Out<Has>
    FOR req IN reqs {
        AddN<User>({ name: user.name, age: user.age })
        skills <- N<Skill>(req::ID)::Out<Includes>
    }
    RETURN skills

QUERY CheckJDMatch (jd_id: ID, user_id: ID) =>
    // Get MySkills
    sources <- N<Source>(user_id)::Out<Has>
    FOR source IN sources {
        my_bullets <- N<Bullet>(source::ID)::Out<Contains>
        FOR bullet IN my_bullets {
            my_skills <- N<Skill>(bullet::ID)::Out<Displays>
            //::GROUP_BY(property1, property2, ...)
        }
    }
    // Get JDSkills
    reqs <- N<Req>(jd_id)::Out<Has>
    FOR req IN reqs {
        jd_skills <- N<Skill>(req::ID)::Out<Includes>
    }
    RETURN skills

QUERY CreateUser (name: String, handle: String) =>
    user <- AddN<User>({
        name: name,
        handle: handle,
    })
    RETURN user

QUERY FollowUser (follower_id: ID, followed_id: ID, since: Date) =>
    follow_edge <- AddE<Follows>({
        since: since
    })::From(follower_id)::To(followed_id)
    RETURN follow_edge
