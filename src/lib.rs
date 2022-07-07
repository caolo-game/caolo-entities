#![feature(option_get_or_insert_default)]
#![feature(const_type_id)]

use std::{any::TypeId, collections::HashMap, pin::Pin, ptr::NonNull};

use db::ArchetypeStorage;
use entity_id::EntityId;
use handle_table::EntityIndex;

pub(crate) mod db;
pub mod entity_id;
pub mod handle_table;
pub mod page_table;

#[cfg(test)]
mod world_tests;

pub struct World {
    // TODO: world can be generic over Index
    entity_ids: Pin<Box<EntityIndex>>,
    archetypes: HashMap<TypeHash, Pin<Box<ArchetypeStorage>>>,
}

type TypeHash = u64;

const fn hash_ty<T: 'static>() -> u64 {
    let ty = TypeId::of::<T>();
    // FIXME extreme curse
    //
    let ty: u64 = unsafe { std::mem::transmute(ty) };
    if ty == unsafe { std::mem::transmute(TypeId::of::<()>()) } {
        // ensure that unit type has hash=0
        0
    } else {
        ty
    }
}

const VOID_TY: TypeHash = hash_ty::<()>();

#[derive(Clone, Debug, thiserror::Error)]
pub enum WorldError {
    #[error("World is full and can not take more entities")]
    OutOfCapacity,
    #[error("Entity was not found")]
    EntityNotFound,
    #[error("Entity doesn't have specified component")]
    ComponentNotFound,
}

pub type WorldResult<T> = Result<T, WorldError>;
pub type RowIndex = u32;

/// The end goal is to have a clonable ECS, that's why we have the Clone restriction.
pub trait Component: 'static + Clone {}
impl<T: 'static + Clone> Component for T {}

pub trait Index {
    type Id;
    type Error;

    fn allocate(&mut self) -> Result<Self::Id, Self::Error>;
    fn update(
        &mut self,
        id: Self::Id,
        payload: (NonNull<ArchetypeStorage>, RowIndex),
    ) -> Result<(), Self::Error>;
    fn delete(&mut self, id: Self::Id) -> Result<(), Self::Error>;
    fn read(&self, id: Self::Id) -> Result<(NonNull<ArchetypeStorage>, RowIndex), Self::Error>;
}

impl World {
    pub fn new(capacity: u32) -> Self {
        let entity_ids = Box::pin(EntityIndex::new(capacity));

        let mut archetypes = HashMap::with_capacity(128);
        let void_store = Box::pin(ArchetypeStorage::empty());
        archetypes.insert(VOID_TY, void_store);

        // FIXME: can't add assert to const fn...
        // the `hash_ty` function assumes that TypeId is a u64 under the hood
        debug_assert_eq!(std::mem::size_of::<TypeId>(), std::mem::size_of::<u64>());
        Self {
            entity_ids,
            archetypes,
        }
    }

    pub fn insert_entity(&mut self) -> WorldResult<EntityId> {
        let id = self
            .entity_ids
            .allocate()
            .map_err(|_| WorldError::OutOfCapacity)?;
        let void_store = self.archetypes.get_mut(&VOID_TY).unwrap();

        let index = void_store.as_mut().insert_entity(id);
        self.entity_ids
            .update(
                id,
                (
                    NonNull::new(void_store.as_mut().get_mut() as *mut _).unwrap(),
                    index,
                ),
            )
            .unwrap();
        Ok(id)
    }

    pub fn delete_entity(&mut self, id: EntityId) -> WorldResult<()> {
        let (mut archetype, index) = self
            .entity_ids
            .read(id)
            .map_err(|_| WorldError::EntityNotFound)?;
        unsafe {
            archetype.as_mut().remove(index);
            self.entity_ids.delete(id).unwrap();
        }
        Ok(())
    }

    pub fn set_component<T: Component>(
        &mut self,
        entity_id: EntityId,
        component: T,
    ) -> WorldResult<()> {
        let (mut archetype, mut index) = self
            .entity_ids
            .read(entity_id)
            .map_err(|_| WorldError::EntityNotFound)?;
        let mut archetype = unsafe { archetype.as_mut() };
        if !archetype.contains_column::<T>() {
            let new_ty = archetype.extended_hash::<T>();
            if !self.archetypes.contains_key(&new_ty) {
                let mut res = self.insert_archetype::<T>(
                    archetype,
                    index,
                    archetype.extend_with_column::<T>(),
                );
                archetype = unsafe { res.as_mut() };
                index = 0;
            } else {
                let new_arch = self.archetypes.get_mut(&new_ty).unwrap();
                index = archetype.move_entity(new_arch, index);
                archetype = new_arch.as_mut().get_mut();
            }
        }
        archetype.set_component(entity_id, index, component);
        unsafe {
            self.entity_ids
                .update(
                    entity_id,
                    (NonNull::new_unchecked(archetype as *mut _), index),
                )
                .unwrap();
        }
        Ok(())
    }

    pub fn get_component<T: Component>(&self, entity_id: EntityId) -> Option<&T> {
        let (arch, idx) = self.entity_ids.read(entity_id).ok()?;
        unsafe { arch.as_ref().get_component(idx) }
    }

    pub fn remove_component<T: Component>(&mut self, entity_id: EntityId) -> WorldResult<()> {
        let (mut archetype, mut index) = self
            .entity_ids
            .read(entity_id)
            .map_err(|_| WorldError::EntityNotFound)?;
        let mut archetype = unsafe { archetype.as_mut() };
        if !archetype.contains_column::<T>() {
            return Err(WorldError::ComponentNotFound);
        }
        let new_ty = archetype.extended_hash::<T>();
        if !self.archetypes.contains_key(&new_ty) {
            let mut res =
                self.insert_archetype::<T>(archetype, index, archetype.reduce_with_column::<T>());
            archetype = unsafe { res.as_mut() };
            index = 0;
        } else {
            let new_arch = self.archetypes.get_mut(&new_ty).unwrap();
            index = archetype.move_entity(new_arch, index);
            archetype = new_arch.as_mut().get_mut();
        }
        unsafe {
            self.entity_ids
                .update(
                    entity_id,
                    (NonNull::new_unchecked(archetype as *mut _), index),
                )
                .unwrap();
        }
        Ok(())
    }

    #[inline(never)]
    fn insert_archetype<T: Component>(
        &mut self,
        archetype: &mut ArchetypeStorage,
        row_index: RowIndex,
        new_arch: ArchetypeStorage,
    ) -> NonNull<ArchetypeStorage> {
        let mut new_arch = Box::pin(new_arch);
        let index = archetype.move_entity(&mut new_arch, row_index);
        debug_assert_eq!(index, 0);
        let res = unsafe { NonNull::new_unchecked(new_arch.as_mut().get_mut() as *mut _) };
        self.archetypes.insert(new_arch.ty(), new_arch);
        res
    }
}
