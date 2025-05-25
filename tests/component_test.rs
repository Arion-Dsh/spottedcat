
#[test]
fn test_component() {
    use ecs::component;
    use ecs::components::Component;

    #[component]
    #[derive(Default, Clone, Debug)]
    struct Position {
        x: f32,
        y: f32,
    }

    struct Player {
        p: Position,
    }

    let pos = Position::default();
    let player = Player { p: pos.clone() };
    let _ = player.p.set_x(1.0);
    let _ = player.p.set_y(2.0);

    assert_eq!(pos.get_x().unwrap(), player.p.get_x().unwrap());
    //
    assert_eq!(pos.get_y().unwrap(), player.p.get_y().unwrap());

}
