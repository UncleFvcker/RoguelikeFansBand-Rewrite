// SPDX-License-Identifier: MPL-2.0
// Adapted from RFB master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
// src/cmd1.c run_init/run_test/run_step. See LICENSES/RFB-UPSTREAM-NOTICE.txt.
// Stop points and corner cutting consume the global behavior context.

use super::*;
use rfb_protocol::RunningStateDto;

const CYCLE: [Direction; 8] = [
    Direction::SouthWest,
    Direction::South,
    Direction::SouthEast,
    Direction::East,
    Direction::NorthEast,
    Direction::North,
    Direction::NorthWest,
    Direction::West,
];

fn rotate(direction: Direction, offset: i32) -> Direction {
    let index = CYCLE.iter().position(|d| *d == direction).unwrap() as i32;
    CYCLE[(index + offset).rem_euclid(8) as usize]
}

fn diagonal(direction: Direction) -> bool {
    let (x, y) = direction.delta();
    x != 0 && y != 0
}

fn neighbor(position: Position, direction: Direction) -> Position {
    let (dx, dy) = direction.delta();
    Position {
        x: position.x + dx,
        y: position.y + dy,
    }
}

impl Game {
    pub(super) fn running_state_is_valid(&self, run: &RunningStateDto) -> bool {
        self.map_scale == MapScaleDto::Local
            && run.floor_id == self.current_floor_id
            && run.position == self.player.position
            && (1..9999).contains(&run.remaining_steps)
            && run.origin.x.abs_diff(run.position.x) <= 9999
            && run.origin.y.abs_diff(run.position.y) <= 9999
    }

    fn run_wall(&self, position: Position, ignore_avoid_run: bool) -> bool {
        let Some(index) = self.index(position) else {
            return true;
        };
        if !self.explored[index] {
            return false;
        }
        let terrain = self
            .content
            .terrain(self.known_terrain_at(position))
            .unwrap();
        if terrain.open_to_terrain_id.is_some() || terrain.close_to_terrain_id.is_some() {
            return false;
        }
        let can_enter = if self.is_wilderness_floor() {
            self.player_can_cross_surface_terrain(terrain)
        } else {
            self.player_can_cross_terrain(terrain)
        };
        !can_enter
            || (!ignore_avoid_run && Self::terrain_avoids_running(terrain))
            || (!terrain.walkable && terrain.movement_modes.is_empty())
    }

    fn terrain_avoids_running(terrain: &rfb_content::TerrainDefinition) -> bool {
        terrain
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "tree" | "mountain"))
    }

    fn run_interesting(&self, position: Position) -> bool {
        if self.entities.iter().any(|entity| {
            entity.hp > 0
                && !self.actor_is_player_side(entity)
                && entity.position == position
                && self.entity_is_visible_to_player(entity)
        }) || self.items.iter().any(|item| {
            matches!(&item.location, ItemLocation::Ground(p) if *p == position)
                && self.item_is_discovered(&item.id)
        }) || self
            .gold_piles
            .iter()
            .any(|pile| pile.position == position && pile.discovered)
        {
            return true;
        }
        let Some(index) = self.index(position) else {
            return false;
        };
        if !self.explored[index] {
            return false;
        }
        let terrain = self
            .content
            .terrain(self.known_terrain_at(position))
            .unwrap();
        let tagged = |tag| terrain.tags.iter().any(|t| t == tag);
        if terrain.trap.is_some()
            || terrain.open_to_terrain_id.is_some()
            || (self.operation_options.run_stops.stairs
                && (tagged("stairs-up")
                    || tagged("stairs-down")
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)))
            || (self.operation_options.run_stops.known_treasure
                && tagged("vein")
                && tagged("treasure"))
        {
            return true;
        }
        if terrain.close_to_terrain_id.is_some() {
            return self.operation_options.run_stops.open_doors;
        }
        if tagged("lava") {
            return !self.player_has_status_kind(STATUS_INVULNERABILITY)
                && self.effective_player_resistances().level(DamageType::Fire)
                    != ResistanceLevel::Immune;
        }
        if tagged("water") && tagged("deep") {
            use rfb_content::ActorMovementMode;
            return !self.active_traveler_has_mode(ActorMovementMode::Fly)
                && !self.active_traveler_has_mode(ActorMovementMode::Swim)
                && !self.active_traveler_has_mode(ActorMovementMode::Aquatic)
                && self.carried_weight_tenths_pound() > self.player_carry_capacity_tenths_pound();
        }
        tagged("acid")
            || tagged("building")
            || tagged("shop-entrance")
            || tagged("town-facility-entrance")
            || tagged("task-entry")
            || tagged("dungeon-entrance")
    }

    fn new_run(&self, direction: Direction, max_steps: u16) -> RunningStateDto {
        let position = self.player.position;
        let next = neighbor(position, direction);
        let ignore_avoid_run = self.index(next).is_some_and(|index| {
            self.explored[index]
                && Self::terrain_avoids_running(
                    self.content.terrain(self.known_terrain_at(next)).unwrap(),
                )
        });
        let short_left = self.run_wall(neighbor(position, rotate(direction, 1)), ignore_avoid_run);
        let short_right =
            self.run_wall(neighbor(position, rotate(direction, -1)), ignore_avoid_run);
        let deep_left =
            !short_left && self.run_wall(neighbor(next, rotate(direction, 1)), ignore_avoid_run);
        let deep_right =
            !short_right && self.run_wall(neighbor(next, rotate(direction, -1)), ignore_avoid_run);
        let break_left = short_left || deep_left;
        let break_right = short_right || deep_right;
        let mut previous_direction = direction;
        if break_left && break_right {
            if diagonal(direction) {
                if deep_left && !deep_right {
                    previous_direction = rotate(direction, -1);
                } else if deep_right && !deep_left {
                    previous_direction = rotate(direction, 1);
                }
            } else if self.run_wall(neighbor(next, direction), ignore_avoid_run) {
                if short_left && !short_right {
                    previous_direction = rotate(direction, -2);
                } else if short_right && !short_left {
                    previous_direction = rotate(direction, 2);
                }
            }
        }
        RunningStateDto {
            direction,
            previous_direction,
            open_area: !(break_left && break_right),
            ignore_avoid_run,
            break_left,
            break_right,
            origin: position,
            position,
            floor_id: self.current_floor_id.clone(),
            remaining_steps: max_steps,
        }
    }

    fn next_run_direction(&self, run: &mut RunningStateDto) -> Result<Direction, &'static str> {
        let previous = run.previous_direction;
        let max = if diagonal(previous) { 2 } else { 1 };
        let mut option = None;
        let mut option2 = None;
        let mut check_direction = previous;
        for offset in -max..=max {
            let direction = rotate(previous, offset);
            let position = neighbor(self.player.position, direction);
            if self.run_interesting(position) {
                return Err("game-run-stopped-interesting");
            }
            if !self.run_wall(position, run.ignore_avoid_run) {
                if !run.open_area {
                    if option.is_none() {
                        option = Some(direction);
                    } else if option2.is_some() || option != Some(rotate(previous, offset - 1)) {
                        return Err("game-run-stopped-junction");
                    } else if diagonal(direction) {
                        check_direction = rotate(previous, offset - 2);
                        option2 = Some(direction);
                    } else {
                        check_direction = rotate(previous, offset + 1);
                        option2 = option;
                        option = Some(direction);
                    }
                }
            } else if run.open_area {
                if offset < 0 {
                    run.break_right = true;
                } else if offset > 0 {
                    run.break_left = true;
                }
            }
        }
        if run.open_area {
            for offset in -max..0 {
                let wall = self.run_wall(
                    neighbor(self.player.position, rotate(previous, offset)),
                    run.ignore_avoid_run,
                );
                if (!wall && run.break_right) || (wall && run.break_left) {
                    return Err("game-run-stopped-junction");
                }
            }
            for offset in (1..=max).rev() {
                let wall = self.run_wall(
                    neighbor(self.player.position, rotate(previous, offset)),
                    run.ignore_avoid_run,
                );
                if (!wall && run.break_left) || (wall && run.break_right) {
                    return Err("game-run-stopped-junction");
                }
            }
        } else {
            run.direction = option.ok_or("game-run-stopped-blocked")?;
            run.previous_direction = option2.unwrap_or(run.direction);
            if self.operation_options.cut_corners
                && let Some(corner) = option2
            {
                let next = neighbor(self.player.position, run.direction);
                if self.run_wall(neighbor(next, run.direction), run.ignore_avoid_run)
                    && self.run_wall(neighbor(next, check_direction), run.ignore_avoid_run)
                {
                    run.direction = corner;
                    run.previous_direction = corner;
                } else {
                    let unknown = |direction| {
                        self.index(neighbor(next, direction))
                            .is_some_and(|index| !self.explored[index])
                    };
                    if !unknown(run.direction) || !unknown(corner) {
                        return Err("game-run-stopped-junction");
                    }
                }
            }
        }
        if self.run_wall(
            neighbor(self.player.position, run.direction),
            run.ignore_avoid_run,
        ) {
            return Err("game-run-stopped-blocked");
        }
        Ok(run.direction)
    }

    pub(super) fn prepare_run(
        &mut self,
        action: &GameAction,
        events: &mut Vec<DomainEvent>,
    ) -> Option<Direction> {
        if !matches!(action, GameAction::Run { .. } | GameAction::ContinueRun) {
            self.running = None;
            return None;
        }
        let result = (|| {
            if self.map_scale != MapScaleDto::Local
                || self.player_is_dead()
                || self.player_has_status_kind(STATUS_BLINDNESS)
                || self.player_has_status_kind(STATUS_CONFUSION)
                || self.player_has_status_kind(STATUS_PARALYSIS)
            {
                return Err("game-run-stopped-condition");
            }
            let mut run = match action {
                GameAction::Run {
                    direction,
                    max_steps,
                } => {
                    if !(1..=9999).contains(max_steps) {
                        return Err("game-run-stopped-limit");
                    }
                    if self.run_wall(neighbor(self.player.position, *direction), true) {
                        return Err("game-run-stopped-blocked");
                    }
                    self.new_run(*direction, *max_steps)
                }
                _ => self
                    .running
                    .take()
                    .filter(|run| self.running_state_is_valid(run))
                    .ok_or("game-run-stopped-condition")?,
            };
            let direction = if matches!(action, GameAction::Run { .. }) {
                run.direction
            } else {
                self.next_run_direction(&mut run)?
            };
            run.remaining_steps -= 1;
            self.running = Some(run);
            Ok(direction)
        })();
        match result {
            Ok(direction) => Some(direction),
            Err(reason) => {
                self.running = None;
                events.push(DomainEvent::RunStopped { reason });
                None
            }
        }
    }

    pub(super) fn finish_run_step(
        &mut self,
        moved: bool,
        translation: Option<Position>,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some(mut run) = self.running.take() else {
            return;
        };
        let mut expected_position = neighbor(run.position, run.direction);
        if let Some(translation) = translation {
            run.origin.x += translation.x;
            run.origin.y += translation.y;
            expected_position.x += translation.x;
            expected_position.y += translation.y;
        }
        run.position = self.player.position;
        if !moved
            || run.position != expected_position
            || run.position == run.origin
            || run.floor_id != self.current_floor_id
            || self.map_scale != MapScaleDto::Local
            || self.player_is_dead()
            || self.player_has_status_kind(STATUS_BLINDNESS)
            || self.player_has_status_kind(STATUS_CONFUSION)
            || self.player_has_status_kind(STATUS_PARALYSIS)
            || self.mogaminator.pending_query.is_some()
            || self.pending_duelist.is_some()
            || self.pending_mutation_direction.is_some()
            || self.pending_ability_direction.is_some()
        {
            events.push(DomainEvent::RunStopped {
                reason: "game-run-stopped-condition",
            });
        } else if run.remaining_steps == 0 {
            events.push(DomainEvent::RunStopped {
                reason: "game-run-stopped-limit",
            });
        } else {
            self.running = Some(run);
        }
    }
}
