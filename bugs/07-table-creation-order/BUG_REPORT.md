# CDM Bug Report: Tables created before their dependencies

## Summary

When generating SQL migrations that create multiple tables with foreign key relationships, the SQL plugin creates tables in the wrong order. Tables are created before the tables they reference, causing the migration to fail with foreign key constraint violations.

## Real-world Example

In the actual migration (`0002.up.postgres.sql`):

```sql
-- project_repos is created FIRST
CREATE TABLE "project_repos" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "project_id" UUID NOT NULL REFERENCES "projects"("id") ON DELETE CASCADE,  -- ERROR: projects doesn't exist yet!
  ...
);

-- daemons is created SECOND (depends on project_repos)
CREATE TABLE "daemons" (
  "id" UUID NOT NULL,
  "project_repo_id" UUID NOT NULL REFERENCES "project_repos"("id") ON DELETE CASCADE,
  ...
);

-- projects is created THIRD (but should be FIRST!)
CREATE TABLE "projects" (
  "id" UUID NOT NULL,
  ...
);
```

The dependency chain is: `projects` <- `project_repos` <- `daemons`

But the tables are created in order: `project_repos`, `daemons`, `projects`

This causes the migration to fail because `project_repos` references `projects` which doesn't exist yet.

## Root Cause

The SQL migration plugin does not perform topological sorting based on foreign key dependencies. Tables appear to be generated in alphabetical or arbitrary order rather than dependency order.

## Minimal Reproduction

### Files

**`public.cdm`**:
```cdm
import sqlType from "sql-types/postgres"

@sql

Entity {
  id: sqlType.UUID
  @sql {
    indexes: [
      { fields: ["id"], primary: true }
    ]
  }
}

Timestamped {
  created_at: sqlType.TimestampTZ {
    @sql { default: "NOW()" }
  }
}

TimestampedEntity extends Entity, Timestamped {
}

PublicUser extends TimestampedEntity {
  name?: string
}

// Project depends on User (owner_id)
PublicProject extends TimestampedEntity {
  name: string
  owner_id: string
}

// ProjectRepo depends on Project (project_id)
PublicProjectRepo extends TimestampedEntity {
  project_id: string
  repo_url: string
}

// Daemon depends on ProjectRepo (project_repo_id)
PublicDaemon extends TimestampedEntity {
  project_repo_id: string
  machine_name?: string
}
```

**`database.cdm`**:
```cdm
import sql from "sql-types/postgres"

extends "./public.cdm"

@sql {
  dialect: "postgresql",
  migrations_output: "./migrations",
  build_output: "./output"
}

-PublicUser
-Timestamped
-Entity
-TimestampedEntity
-PublicProject
-PublicProjectRepo
-PublicDaemon

User extends PublicUser {
  email?: sql.Varchar
}

// Dependency chain: User <- Project <- ProjectRepo <- Daemon
// Tables should be created in order: projects, project_repos, daemons

Project extends PublicProject {
  owner_id: sql.UUID {
    @sql {
      references: { table: "users", column: "id", on_delete: "cascade" }
    }
  }
}

ProjectRepo extends PublicProjectRepo {
  project_id: sql.UUID {
    @sql {
      references: { table: "projects", column: "id", on_delete: "cascade" }
    }
  }
}

Daemon extends PublicDaemon {
  project_repo_id: sql.UUID {
    @sql {
      references: { table: "project_repos", column: "id", on_delete: "cascade" }
    }
  }
}
```

**`previous_schema.json`** - State before adding new entities (only User table exists):
```json
{
  "models": {
    "User": {
      "name": "User",
      "parents": ["PublicUser"],
      "fields": [
        { "name": "id", "field_type": { "type": "identifier", "name": "string" }, "optional": false, "config": { "sql": { "type": "UUID" } } },
        { "name": "created_at", "field_type": { "type": "identifier", "name": "string" }, "optional": false, "config": { "sql": { "default": "NOW()", "type": "TIMESTAMPTZ" } } },
        { "name": "name", "field_type": { "type": "identifier", "name": "string" }, "optional": true, "config": {} },
        { "name": "email", "field_type": { "type": "identifier", "name": "string" }, "optional": true, "config": { "sql": { "type": "VARCHAR" } } }
      ],
      "config": {
        "sql": {
          "indexes": [
            { "fields": ["id"], "primary": true }
          ]
        }
      }
    }
  },
  "type_aliases": {}
}
```

### Steps to Reproduce

```bash
cd packages/schema/cdm-bug-repro/bugs/07-table-creation-order
mkdir -p .cdm
cp previous_schema.json .cdm/previous_schema_database.json
cdm migrate database.cdm --name add_entities
cat migrations/add_entities.up.postgres.sql
```

## Actual Output

```sql
CREATE TABLE "project_repos" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "project_id" UUID NOT NULL REFERENCES "projects"("id") ON DELETE CASCADE,
  "repo_url" VARCHAR(255) NOT NULL
);

CREATE TABLE "daemons" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "project_repo_id" UUID NOT NULL REFERENCES "project_repos"("id") ON DELETE CASCADE,
  "machine_name" VARCHAR(255)
);

CREATE TABLE "projects" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "name" VARCHAR(255) NOT NULL,
  "owner_id" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  PRIMARY KEY ("id")
);
```

Running this migration fails with:
```
ERROR: relation "projects" does not exist
```

## Expected Output

```sql
CREATE TABLE "projects" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "name" VARCHAR(255) NOT NULL,
  "owner_id" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  PRIMARY KEY ("id")
);

CREATE TABLE "project_repos" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "project_id" UUID NOT NULL REFERENCES "projects"("id") ON DELETE CASCADE,
  "repo_url" VARCHAR(255) NOT NULL,
  PRIMARY KEY ("id")
);

CREATE TABLE "daemons" (
  "id" UUID NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "project_repo_id" UUID NOT NULL REFERENCES "project_repos"("id") ON DELETE CASCADE,
  "machine_name" VARCHAR(255),
  PRIMARY KEY ("id")
);
```

Tables should be created in dependency order: first `projects`, then `project_repos`, then `daemons`.

## Notes

- This is a critical bug that prevents migrations from running
- The fix requires topological sorting of tables based on foreign key references
- Circular dependencies should be detected and reported as an error
- An alternative fix would be to create tables first, then add foreign keys via ALTER TABLE
