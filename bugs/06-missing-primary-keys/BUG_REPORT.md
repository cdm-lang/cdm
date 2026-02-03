# CDM Bug Report: Missing PRIMARY KEY constraints on new tables

## Summary

When adding new entities that extend `TimestampedEntity` (which extends `Entity` with a primary key index), the SQL migration plugin does not generate `PRIMARY KEY` constraints for the new tables. Some tables get the PRIMARY KEY while others do not, despite having identical inheritance chains.

## Real-world Example

In the actual migration (`0002.up.postgres.sql`), three tables are missing PRIMARY KEY constraints:

```sql
CREATE TABLE "project_repos" (
  "id" UUID NOT NULL,
  ...
  -- Missing: PRIMARY KEY ("id")
);

CREATE TABLE "daemons" (
  "id" UUID NOT NULL,
  ...
  -- Missing: PRIMARY KEY ("id")
);

CREATE TABLE "daemon_auth_requests" (
  "id" UUID NOT NULL,
  ...
  -- Missing: PRIMARY KEY ("id")
);
```

Meanwhile, the `projects` table correctly has:
```sql
CREATE TABLE "projects" (
  "id" UUID NOT NULL,
  ...
  PRIMARY KEY ("id")
);
```

All four entities extend `TimestampedEntity` which extends `Entity`, and `Entity` defines:
```cdm
@sql {
  indexes: [
    { fields: ["id"], primary: true }
  ]
}
```

## Root Cause

The SQL plugin appears to inconsistently inherit the primary key index configuration from parent entities. The same inheritance chain produces different results for different tables.

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

// New entities to add
PublicProject extends TimestampedEntity {
  name: string
}

PublicProjectRepo extends TimestampedEntity {
  project_id: string
  repo_url: string
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

User extends PublicUser {
  email?: sql.Varchar
}

// New entities - both extend TimestampedEntity which extends Entity
// Entity defines primary key index on "id"
// Expected: PRIMARY KEY ("id") should be generated

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
```

**`previous_schema.json`** - State before adding new entities (User table exists with PRIMARY KEY):
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
cd packages/schema/cdm-bug-repro/bugs/06-missing-primary-keys
mkdir -p .cdm
cp previous_schema.json .cdm/previous_schema_database.json
cdm migrate database.cdm --name add_entities
cat migrations/add_entities.up.postgres.sql
```

## Actual Output

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
  "repo_url" VARCHAR(255) NOT NULL
);
```

Note: `projects` has PRIMARY KEY but `project_repos` does not.

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
```

Both tables should have `PRIMARY KEY ("id")` since they both inherit from `Entity` which defines the primary key index.

## Notes

- This is a critical bug as tables without primary keys can have serious database integrity issues
- The inconsistency suggests a race condition or order-dependent behavior in how configs are inherited
- The User table (created in a previous migration) correctly has the PRIMARY KEY
