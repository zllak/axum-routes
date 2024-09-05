You want to avoid as much as possible to hardcode your routes when
developping a web app. It is so easy to put a typo in it, and you end up
with broken links in your app.

You can use the [`resolve!`] macro to generate your routes.
You have compile-time validation of the types of parameters you pass
(any type that implements the [`ToString`] trait), and run-time
validation of the number of parameters you pass ([`resolve!`] returns an error
if the number of parameters is not valid).


Here, we resolve a nested route with one parameter:

```no_test,rust
let resolved = axum_routes::resolve!(Routes::Users(RoutesUsers::GetByID), 42).expect("should not fail");
assert_eq!("/users/42", resolved);
```

This will resolve the whole path, starting at the root node (Users enum),
to the GetByID route.
But you can also use the nested route alone:

```no_test,rust
let resolved = axum_routes::resolve!(RoutesUsers::GetByID, 42).expect("should not fail");
assert_eq!("/42", resolved);
```

If the final route has multiple parameters, and that you resolve the whole
route, you need to pass all parameters, in the right order:

```no_test,rust
let user_id = 42;
let resource_id = 21;
let resolved = axum_routes::resolve!(Routes::Users(RoutesUsers::GetOtherResourceByID), user_id, resource_id).expect("should not fail");
assert_eq!("/users/42/other/21", resolved);
```

If you pass the wrong number of arguments, the macro will return an error:

```no_test,rust
let resolved = axum_routes::resolve!(RoutesUsers::GetByID, 42, 21).expect("this will fail");
```
