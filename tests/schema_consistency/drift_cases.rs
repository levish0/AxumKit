pub struct DriftCase {
    pub statements: &'static [&'static str],
    pub expected_problem: &'static str,
}

pub const CASES: &[DriftCase] = &[
    DriftCase {
        statements: &["DROP TABLE public.profiles"],
        expected_problem: "profiles: missing database table",
    },
    DriftCase {
        statements: &["CREATE TABLE public.unregistered_table ()"],
        expected_problem: "unregistered_table: missing entity",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts DROP COLUMN title"],
        expected_problem: "posts.title: missing database column",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts ADD COLUMN extra text"],
        expected_problem: "posts.extra: missing entity column",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts ALTER COLUMN title TYPE varchar(100)"],
        expected_problem: "posts.title: type mismatch",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.api_clients ALTER COLUMN scopes TYPE json USING scopes::json",
        ],
        expected_problem: "api_clients.scopes: type mismatch",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts ALTER COLUMN title DROP NOT NULL"],
        expected_problem: "posts.title: nullable mismatch",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts DROP CONSTRAINT posts_pkey CASCADE"],
        expected_problem: "posts.id: primary key position mismatch",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.post_attachments DROP CONSTRAINT post_attachments_pkey",
            "ALTER TABLE public.post_attachments ADD PRIMARY KEY (post_id, file_id)",
            "ALTER TABLE entity_reference.post_attachments DROP CONSTRAINT post_attachments_pkey",
            "ALTER TABLE entity_reference.post_attachments ADD PRIMARY KEY (file_id, post_id)",
        ],
        expected_problem: "post_attachments.post_id: primary key position mismatch",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts DROP CONSTRAINT posts_state_check"],
        expected_problem: "posts.state: missing enum CHECK",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.posts DROP CONSTRAINT posts_state_check",
            "ALTER TABLE public.posts ADD CHECK (state IN ('draft', 'published', 'deleted', 'archived'))",
        ],
        expected_problem: "database-only values [\"archived\"]",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.posts DROP CONSTRAINT posts_state_check",
            "ALTER TABLE public.posts ADD CHECK (state IN ('draft', 'published'))",
        ],
        expected_problem: "Rust-only values [\"deleted\"]",
    },
    DriftCase {
        statements: &["ALTER TABLE public.posts ADD CHECK (title IN ('one', 'two'))"],
        expected_problem: "posts.title: unregistered enum CHECK",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.posts DROP CONSTRAINT posts_state_check",
            "ALTER TABLE public.posts ADD CHECK (state IN ('draft', 'published', 'deleted') OR state = 'archived')",
        ],
        expected_problem: "posts.state: unsupported enum CHECK",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.posts DROP CONSTRAINT posts_state_check",
            "ALTER TABLE public.posts ADD CHECK (state IN ('draft', 'published', 'deleted')) NOT VALID",
        ],
        expected_problem: "posts.state: enum CHECK is not validated",
    },
    DriftCase {
        statements: &[
            "ALTER TABLE public.posts ADD CHECK (state IN ('draft', 'published', 'deleted'))",
        ],
        expected_problem: "posts.state: multiple enum CHECK constraints",
    },
];
