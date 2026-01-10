# Task App

A full-stack task management application built with CDM (Contextual Data Models).

## Architecture

This is a Yarn 4 monorepo with the following structure:

```
task-app/
├── apps/
│   ├── web/          # React + Chakra UI + ts-rest client
│   ├── api/          # Fastify + ts-rest + TypeORM
│   └── database/     # TypeORM migrations runner
├── packages/
│   └── cdm/          # CDM schemas with multiple contexts
└── package.json      # Yarn 4 workspace root
```

## CDM Contexts

- `packages/cdm/base.cdm` - Core models (User, Task, Session)
- `packages/cdm/database.cdm` - Full schema for DB migrations
- `packages/cdm/api.cdm` - API-safe view (removes password_hash)
- `packages/cdm/client.cdm` - Client-safe view + API DTOs

## Prerequisites

- Node.js 18+
- PostgreSQL 14+
- Yarn 4

## Setup

1. Install dependencies:
   ```bash
   yarn install
   ```

2. Create a PostgreSQL database:
   ```sql
   CREATE DATABASE taskapp;
   ```

3. Configure environment variables:
   ```bash
   # apps/api/.env
   DB_HOST=localhost
   DB_PORT=5432
   DB_USER=postgres
   DB_PASSWORD=postgres
   DB_NAME=taskapp
   COOKIE_SECRET=your-secret-key
   ```

4. Run database migrations:
   ```bash
   yarn db:migrate:up
   ```

## Development

Start the development servers:

```bash
yarn dev
```

This will start:
- API server at http://localhost:3001
- Web app at http://localhost:5173

## Building

Build all packages:

```bash
yarn build
```

## Features

- User authentication (register, login, logout)
- Task CRUD operations
- Task status management (todo, in_progress, done)
- Session-based authentication with secure cookies
- Form validation using JSON Schema
- Type-safe API contracts with ts-rest

## Tech Stack

### Frontend
- React 18
- Chakra UI
- React Query
- ts-rest
- react-jsonschema-form

### Backend
- Fastify
- TypeORM
- ts-rest
- bcrypt
- PostgreSQL

### Shared
- TypeScript
- Zod
- CDM schemas
