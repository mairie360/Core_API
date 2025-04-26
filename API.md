# 📚 API

---

## 👤 Users

| Method | Path | Description |
|:------|:----|:------------|
| `POST` | `/users` | Create a new user |
| `GET` | `/users` | List all users |
| `GET` | `/users/:user_id` | Get a single user |
| `PATCH` | `/users/:user_id` | Update user info (email, password) |
| `DELETE` | `/users/:user_id` | Delete user |
| `POST` | `/users/:user_id/roles/:role_id` | Assign a role to a user |
| `DELETE` | `/users/:user_id/roles/:role_id` | Remove a role from a user |
| `GET` | `/users/:user_id/roles` | List all roles of a user |

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
| `POST` | `/roles` | Create a new role |
| `GET` | `/roles` | List all roles |
| `GET` | `/roles/:role_id` | Get role details |
| `PATCH` | `/roles/:role_id` | Update role |
| `DELETE` | `/roles/:role_id` | Delete role |
| `POST` | `/roles/:roleId/permissions/:permissionId` | Assign a permission to a role |
| `DELETE` | `/roles/:roleId/permissions/:permissionId` | Remove a permission from a role |
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
