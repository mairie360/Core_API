# 📘 Database

This document describes the PostgreSQL schema for the **Core module**, which handles:

- Authentication  
- Authorization (users, roles, permissions)  
- Module registration and management  

---

## 🧭 Overview

The schema includes tables for users, roles, permissions, and modules, with many-to-many linking tables for flexible assignment.

---

## 📘 Full PostgreSQL Schema for Core Module (Multi-API Ready)

### 🔄 Triggers (Auto-update `updated_at`)

```sql
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE 'plpgsql';
```

---

## 🧭 Core Tables

---

### `users`

```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(320) UNIQUE NOT NULL,
    password CHAR(60) NOT NULL, -- bcrypt hash
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);
```

```sql
CREATE TRIGGER trg_users_updated_at
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

---

### `sessions`

```sql
CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,
    user_id INT REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);
```

---

### `roles`

```sql
CREATE TABLE roles (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) UNIQUE NOT NULL,
    description TEXT
);
```

---

### `user_roles`

```sql
CREATE TABLE user_roles (
    user_id INT REFERENCES users(id) ON DELETE CASCADE,
    role_id INT REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);
```

---

### `resources`

```sql
CREATE TABLE resources (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);
```

```sql
CREATE TRIGGER trg_resources_updated_at
BEFORE UPDATE ON resources
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

---

### `permissions`

```sql
CREATE TABLE permissions (
    id SERIAL PRIMARY KEY,
    resource_id INT REFERENCES resources(id) ON DELETE CASCADE,
    action VARCHAR(16) NOT NULL CHECK (action IN ('create', 'read', 'update', 'delete', 'read_all', 'update_all', 'delete_all')),
    description TEXT
);
```

---

### `role_permissions`

```sql
CREATE TABLE role_permissions (
    role_id INT REFERENCES roles(id) ON DELETE CASCADE,
    permission_id INT REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);
```