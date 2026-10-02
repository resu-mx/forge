# Production Databases

- Contacts/Relations (Graph Database)
- Organizations
- Job Descriptions
- Metadata Index
  - Domains
  - Archetypes
  - Skills
  - Credentials
  - Certifications
  - Courses
  - Degrees
- Skills (GraphDB)
- Vectors
  - Domains
  - Archetypes
  - Skills
  - Credentials
  - Certifications
  - Courses
  - Organizations
  - Degrees


## Client Side

- [RxDB](https://rxdb.info/overview.html)
- *SatoriDB*: An embedded, billion-scale vector database that runs entirely in-process and is optimized for datasets larger than available RAM; focuses on predictable latency and high recall for billion-scale datasets.
- *SahomeDB*: A lightweight, SQLite-inspired embedded database that uses the Sled engine for persistence and HNSW for indexing.
- *PolarisDB*: A local-first vector search engine designed for developers who need fast search without managing external services.
- *TinyVector*: A minimal embedding database (approx. 600 lines of code) built on the Axum web framework.
- *LanceDB*: An open-source, in-process database built on the Apache Arrow format. It is designed for high-performance vector search and is capable of handling terabyte-scale datasets while remaining serverless and production-ready.
- *SurrealDB*: A multi-model database that can be embedded directly in Rust applications. It supports vector embeddings as a first-class type, allowing you to combine graph, document, and vector queries in one process.
- *OasysDB*: A hybrid embedded vector database that allows using relational engines like SQLite for storage while maintaining a fast, isolated vector indexing layer.
- [HelixDB](https://www.helix-db.com/)
- [RuVector](https://github.com/ruvnet/ruvector)

```hql
QUERY GetBulletSkills (bullet_id: ID) =>
    skills <- N<Skill>(bullet_id)::Out<Has>
    RETURN following
```

```hql
QUERY GetJDSkills (jd_id: ID) =>
    reqs <- N<Req>(jd_id)::Out<Has>
    FOR req IN reqs {
        AddN<User>({ name: user.name, age: user.age })
        skills <- N<Skill>(req::ID)::Out<Includes>
    }
    RETURN skills
```

```hql
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
```

```hql
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
```

`/wrap` "capture our progress as a memory and output a prompt for triggering the next agent to take over where we left off"
