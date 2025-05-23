The [`router!`] macro lets you create and customize the [`axum::Router`] from
your defined enums.

First, let's define our router as an enum:
```ignore,rust
#[routes]
enum MyRoutes {
    #[get("/admin", handler = admin_handler)]
    ProtectedAdmin,
    #[get("/assets/{asset}", handler = assets)]
    Assets,
}

fn main() {
    let router: axum::Router = axum_routes::router!(MyRoutes);

    // .. serve the Router
}
```

### Applying layer, route_layer, fallback, with_state, ..

Now, we can modify our enum to let `axum-routes` know that we want to customize
the routes, like applying layers, setting the fallback handler, add a state with
with_state. (See documentation of [MethodRouter](`axum::routing::method_routing::MethodRouter`)).

```ignore,rust
#[routes]
enum MyRoutes {
    #[get("/admin", handler = admin_handler, customize = protected_admin)]
    ProtectedAdmin,
    #[get("/assets/{asset}", handler = assets, customize = assets_customize)]
    Assets,
}
```

You can see that we use the `customize` identifier for the ProtectedAdmin
and Assets variants.

So, when we generate the [`axum::Router`] associated with the `MyRoutes` enum,
we use the [`router!`] macro and pass the customizers to it.

```ignore,rust
let router = axum_routes::router!(
    self::MyRoutes,
    protected_admin = ${
        // We do have a `protected_layer` that we will share using Arc
        let layer = Arc::clone(protected_layer);
        move |route| {
            route.layer(layer)
        }
    },
    assets_customize = $|route| {
        // For example, apply a rate limiter layer (must be declared above)
        route.layer(my_rate_limiter_layer)
    },
);
```

Note the `$` before the expression that return the closure, it is used
to tell `axum-routes` that this will customize a [MethodRouter](`axum::routing::method_routing::MethodRouter`).

This also works when nesting routers:

```ignore,rust
#[routes]
enum MyRoutes {
    #[nest("/nested", customize = custom_nested)]
    Nested(OtherRoutes),
}

#[routes]
enum OtherRoutes {
    #[get("/test", handler = handler)]
    Test,
}

fn main() {
    let router = axum_routes::router!(
        self::MyRoutes,
        custom_nested = #|router| {
            router.with_state(State::new())
                .fallback(fallback_handler)
                .layer(ratelimiter)
        },
    );
}
```

Here we are customizing a [`axum::Router`], so we have to use the `#`.
