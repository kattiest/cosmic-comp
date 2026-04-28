// SPDX-License-Identifier: GPL-3.0-only

use crate::state::State;
use crate::utils::prelude::*;
use smithay::{
    delegate_pointer_constraints,
    input::pointer::{MotionEvent, PointerHandle},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        pointer_constraints::{PointerConstraintsHandler, with_pointer_constraint},
        seat::WaylandFocus,
    },
};

impl PointerConstraintsHandler for State {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>) {
        // XXX region
        if pointer
            .current_focus()
            .is_some_and(|x| x.wl_surface().as_deref() == Some(surface))
        {
            with_pointer_constraint(surface, pointer, |constraint| {
                constraint.unwrap().activate();
            });
        }
    }

    fn cursor_position_hint(
        &mut self,
        surface: &WlSurface,
        pointer: &PointerHandle<Self>,
        location: Point<f64, Logical>,
    ) {
        // Only act if the constraint is currently active (i.e. the pointer is locked)
        let is_locked = with_pointer_constraint(surface, pointer, |constraint| {
            constraint.is_some_and(|c| c.is_active())
        });
        if !is_locked {
            return;
        }

        // Find the global position of the surface so we can translate the
        // surface-local hint into a global compositor coordinate.
        let shell = self.common.shell.read();
        let output = shell.seats.last_active().active_output();
        let surface_global_pos = shell
            .outputs()
            .find_map(|o| {
                State::surface_under(pointer.current_location().as_global(), o, &shell)
                    .and_then(|(target, loc)| {
                        if target.wl_surface().as_deref() == Some(surface) {
                            Some(loc)
                        } else {
                            None
                        }
                    })
            })
            .unwrap_or_else(|| pointer.current_location().as_global());
        std::mem::drop(shell);

        // Translate surface-local hint to global coords
        let new_global = surface_global_pos + location.as_global().to_f64();

        let serial = SERIAL_COUNTER.next_serial();
        let shell = self.common.shell.read();
        let under = State::surface_under(new_global, &output, &shell)
            .map(|(target, pos)| (target, pos.as_logical()));
        std::mem::drop(shell);

        pointer.motion(
            self,
            under,
            &MotionEvent {
                location: new_global.as_logical(),
                serial,
                time: self.common.clock.now().as_millis(),
            },
        );
        pointer.frame(self);
    }
}
delegate_pointer_constraints!(State);
