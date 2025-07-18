use super::{ContainerType, Inventory, Storage};

pub struct InventoryIterator<'a> {
    inventory: &'a Inventory,
    curr: u16,
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = (ContainerType, &'a dyn Storage);
    type IntoIter = InventoryIterator<'a>;
    fn into_iter(self) -> InventoryIterator<'a> {
        InventoryIterator {
            inventory: self,
            curr: 0,
        }
    }
}

// TODO: why is this public API? :D
pub fn get_container_type(container_index: u32) -> Option<ContainerType> {
    match container_index {
        // inventory
        0 => Some(ContainerType::Inventory0),
        1 => Some(ContainerType::Inventory1),
        2 => Some(ContainerType::Inventory2),
        3 => Some(ContainerType::Inventory3),

        // armory
        4 => Some(ContainerType::ArmoryOffWeapon),
        5 => Some(ContainerType::ArmoryHead),
        6 => Some(ContainerType::ArmoryBody),
        7 => Some(ContainerType::ArmoryHand),
        8 => Some(ContainerType::ArmoryLeg),
        9 => Some(ContainerType::ArmoryFoot),
        10 => Some(ContainerType::ArmoryEarring),
        11 => Some(ContainerType::ArmoryNeck),
        12 => Some(ContainerType::ArmoryWrist),
        13 => Some(ContainerType::ArmoryRing),
        14 => Some(ContainerType::ArmorySoulCrystal),
        15 => Some(ContainerType::ArmoryWeapon),

        // equipped
        16 => Some(ContainerType::Equipped),

        // currency
        17 => Some(ContainerType::Currency),
        _ => panic!(
            "Inventory iterator invalid or the client sent a very weird packet! {container_index}"
        ),
    }
}

impl<'a> Iterator for InventoryIterator<'a> {
    type Item = (ContainerType, &'a dyn Storage);

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr;
        self.curr += 1;

        if curr >= 18 {
            return None;
        }

        let container_type = get_container_type(curr as u32).unwrap();

        Some((
            container_type,
            self.inventory.get_container(&container_type),
        ))
    }
}
