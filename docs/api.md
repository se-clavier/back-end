1. `HTTP 400 Bad Request`: 无法操作/错误token
2. `HTTP 401 Unauth`: 需要（重新）登陆
3. `HTTP 403 Forbidden`: 需要权限
3. `HTTP 404 Not Found`: 未找到

### POST /login

通过用户名和密码登陆。

#### Request

附带 form:
```form
username: ***
password: ***
```

#### Reponse

返回 `HTTP 200 OK`

正文中附带 message:
1. `ok` 验证成功，会附带 `Set-Cookie` header
2. `failed` 密码错误，会在 3 秒后回应

### GET /logout

登出

#### Reponse

返回 `HTTP 200 OK` 并使 cookie 失效，返回删除 cookie 的 header
