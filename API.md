# 📚 API

---

## 👤 Users

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/users` | Create a new user |
| `GET` | `/users` | List all users |
| `GET` | `/users/:user_id` | Get a single user |
id
---

## 🔐 Sessions (Login / Logout)

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/sessions` | Create a session (Login) |
| `DELETE` | `/sessions/:session_id` | Delete a session (Logout) |
| `GET` | `/sessions` | List active sessions (optional) |

---

## 🛡️ Roles

| Method | Path | Description |
|:------|:----|:------------|

| `GET` | `/roles/:roleId/permissions` | List all permissions of a role |

---

## 📦 Modules

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/modules` | Create a new module |
| `GET` | `/modules` | List all modules |
| `GET` | `/modules/:id` | Get module details |
| `PATCH` | `/modules/:id` | Update module |
| `DELETE` | `/modules/:id` | Delete module |

---

## 📚 Resources (Linked to Modules)

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/resources` | Create a new resource |
| `GET` | `/resources` | List all resources |
| `GET` | `/resources/:id` | Get resource details |
| `PATCH` | `/resources/:id` | Update resource |
| `DELETE` | `/resources/:id` | Delete resource |
| `GET` | `/modules/:moduleId/resources` | List resources of a specific module |

---

## 🎛️ Permissions (Action on Resources)

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/permissions` | Create a new permission |
| `GET` | `/permissions` | List all permissions |
| `GET` | `/permissions/:id` | Get permission details |
| `PATCH` | `/permissions/:id` | Update permission |
| `DELETE` | `/permissions/:id` | Delete permission |
| `GET` | `/resources/:resourceId/permissions` | List permissions for a specific resource |
