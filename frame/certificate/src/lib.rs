//! # Pallet Certificate
//!
//! - [`nicks::Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! Substrate pallet to manage online certificate
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_cert` -
//! * `issue_cert` -
//!

#![cfg_attr(not(feature = "std"), no_std)]

use base58::ToBase58;
use frame_support::{ensure, traits::EnsureOrigin};
use frame_system::ensure_signed;
pub use pallet::*;
// use sp_core::H256;
use sp_runtime::traits::Hash;
use sp_runtime::RuntimeDebug;
use sp_std::{fmt::Debug, prelude::*, vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

use codec::{Decode, Encode};

// type T::AccountId = u32;
// type CertId<T> = T::Hash;
type CertId = [u8; 32];
type IssuedId = Vec<u8>;

#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::Time};
    use frame_system::pallet_prelude::*;
    // use pallet_organization::OrgProvider;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_organization::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The origin which may forcibly set or remove a name. Root can always do this.
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Time used for marking issued certificate.
        type Time: Time;

        /// Who is allowed to create certificate
        type CreatorOrigin: EnsureOrigin<
            Self::Origin,
            Success = (Self::AccountId, Vec<Self::AccountId>),
        >;

        // /// Organization provider
        // type Organization: pallet_organization::OrgProvider<Self>;

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    // #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
    // pub struct OrgDetail<AccountId: Encode + Decode + Clone + Debug + Eq + PartialEq> {
    //     /// Organization name
    //     name: Vec<u8>,

    //     /// Admin of the organization.
    //     admin: AccountId,

    //     /// Whether this organization suspended
    //     is_suspended: bool,
    // }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
    pub struct CertDetail<AccountId: Encode + Decode + Clone + Debug + Eq + PartialEq> {
        /// Certificate name
        pub name: Vec<u8>,

        /// Description about the certificate.
        pub description: Vec<u8>,

        /// Organization ID
        pub org_id: AccountId,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The object already exsits
        AlreadyExists,

        /// Name too long
        TooLong,

        /// Name too short
        TooShort,

        /// Object doesn't exist.
        NotExists,

        /// Origin has no authorization to do this operation
        PermissionDenied,

        /// ID already exists
        IdAlreadyExists,

        /// Unknown error occurred
        Unknown,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",
        T::Balance = "Balance",
        T::AccountId = "T::AccountId"
    )]
    pub enum Event<T: Config> {
        /// Some certificate added.
        CertAdded(u64, CertId, T::AccountId),

        /// Some cert was issued
        ///
        /// param:
        ///     1 - Hash of issued certificate.
        ///     2 - Recipient of certificate.
        CertIssued(IssuedId, Option<T::AccountId>),
    }

    #[pallet::storage]
    pub type Certificates<T: Config> =
        StorageMap<_, Blake2_128Concat, CertId, CertDetail<T::AccountId>>;

    #[derive(Decode, Encode, Clone, Eq, PartialEq, RuntimeDebug)]
    pub struct CertProof<T: Config> {
        pub cert_id: CertId,

        /// Human readable ID representation.
        pub human_id: Vec<u8>,
        pub time: <<T as pallet::Config>::Time as Time>::Moment,
    }

    impl<T: Config> CertProof<T> {
        fn new(
            cert_id: CertId,
            human_id: Vec<u8>,
            time: <<T as pallet::Config>::Time as Time>::Moment,
        ) -> Self {
            CertProof {
                cert_id,
                human_id,
                time,
            }
        }
    }

    /// double map pair of: T::AccountId -> Issue ID -> Proof
    #[pallet::storage]
    #[pallet::getter(fn issued_cert)]
    pub type IssuedCert<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId, // organization id
        Blake2_128Concat,
        IssuedId, // ID of issued certificate
        CertProof<T>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn issued_cert_owner)]
    pub type IssuedCertOwner<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId, // organization id
        Blake2_128Concat,
        T::AccountId,      // acc handler id
        Vec<CertProof<T>>, // proof
    >;

    #[pallet::storage]
    #[pallet::getter(fn account_id_index)]
    pub type AccountIdIndex<T> = StorageValue<_, u32>;

    #[pallet::storage]
    pub type CertIdIndex<T> = StorageValue<_, u64>;

    /// Certificate module declaration.
    // pub struct Module<T: Config> for enum Call where origin: T::Origin {
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        T::AccountId: AsRef<[u8]>,
    {
        /// Create new certificate
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// # </weight>
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_cert())]
        pub(super) fn create_cert(
            origin: OriginFor<T>,
            org_id: T::AccountId,
            name: Vec<u8>,
            description: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            // let sender = ensure_signed(origin)?;

            ensure!(name.len() >= 3, Error::<T>::TooShort);
            ensure!(name.len() <= 100, Error::<T>::TooLong);

            let (sender, org_ids) = T::CreatorOrigin::ensure_origin(origin)?;

            // pastikan origin adalah admin pada organisasi
            ensure!(
                org_ids.iter().any(|id| *id == org_id),
                Error::<T>::PermissionDenied
            );

            let org = <pallet_organization::Module<T>>::organization(&org_id)
                .ok_or(Error::<T>::NotExists)?;

            // ensure admin
            ensure!(&org.admin == &sender, Error::<T>::PermissionDenied);

            let index = Self::increment_index();
            let cert_id: CertId = Self::generate_hash(
                index
                    .to_le_bytes()
                    .iter()
                    .chain(name.iter())
                    .chain(description.iter())
                    .cloned()
                    .collect::<Vec<u8>>(),
            );

            ensure!(
                !Certificates::<T>::contains_key(cert_id),
                Error::<T>::IdAlreadyExists
            );

            Certificates::<T>::insert(
                cert_id,
                CertDetail {
                    name: name.clone(),
                    org_id: org_id.clone(),
                    description: vec![],
                },
            );

            Self::deposit_event(Event::CertAdded(index, cert_id, org_id));

            Ok(().into())
        }

        /// Issue certificate.
        ///
        /// After organization create certificate; admin should be able to
        /// issue certificate to someone.
        ///
        /// The dispatch origin for this call must match `T::ForceOrigin`.
        ///
        /// # <weight>
        /// # </weight>
        #[pallet::weight(70_000_000)]
        pub(super) fn issue_cert(
            origin: OriginFor<T>,
            org_id: T::AccountId,
            cert_id: CertId,
            recipient: Vec<u8>, // handler sure name
            additional_data: Vec<u8>,
            acc_handler: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let _cert = Certificates::<T>::get(cert_id).ok_or(Error::<T>::NotExists)?;

            ensure!(additional_data.len() < 100, Error::<T>::TooLong);

            // ensure admin
            let org = <pallet_organization::Module<T>>::organization(&org_id)
                .ok_or(Error::<T>::Unknown)?;
            ensure!(org.admin == sender, Error::<T>::PermissionDenied);

            Self::ensure_org_access(&sender, &org_id)?;

            // generate issue id
            // this id is unique per user per cert.

            let issue_id: IssuedId = Self::generate_issued_id(
                org_id
                    .as_ref()
                    // .to_le_bytes()
                    .iter()
                    .chain(cert_id.encode().iter())
                    .chain(recipient.iter())
                    .chain(additional_data.iter())
                    .cloned()
                    .collect::<Vec<u8>>(),
            );

            // pastikan belum pernah di-issue
            ensure!(
                !IssuedCert::<T>::contains_key(&org_id, &issue_id),
                Error::<T>::AlreadyExists
            );

            let proof = CertProof::new(cert_id, recipient, <T as pallet::Config>::Time::now());

            if let Some(ref acc_handler) = acc_handler {
                // apabila sudah pernah diisi update isinya
                // dengan ditambahkan sertifikat pada koleksi penerima.
                IssuedCertOwner::<T>::try_mutate(&org_id, acc_handler, |vs| {
                    // let vs = vs.as_mut().ok_or(Error::<T>::Unknown)?;
                    // vs.push((cert_id, desc, T::Time::now()));
                    if let Some(vs) = vs.as_mut() {
                        vs.push(proof.clone());

                        Ok(())
                    } else {
                        Err(Error::<T>::Unknown)
                    }
                })?;
            }

            IssuedCert::<T>::insert(&org_id, &issue_id, proof);

            Self::deposit_event(Event::CertIssued(issue_id, acc_handler));

            Ok(().into())
        }
    }
}

type Organization<T> = pallet_organization::Organization<T>;

/// The main implementation of this Certificate pallet.
impl<T: Config> Pallet<T>{
    /// Get detail of certificate
    ///
    pub fn get(id: &CertId) -> Option<CertDetail<T::AccountId>> {
        Certificates::<T>::get(id)
    }

    /// Memastikan bahwa akun memiliki akses pada organisasi.
    /// bukan hanya akses, ini juga memastikan organisasi dalam posisi tidak suspended.
    fn ensure_org_access(
        who: &T::AccountId,
        org_id: &T::AccountId,
    ) -> Result<Organization<T::AccountId>, Error<T>> {
        let org = pallet_organization::Module::<T>::ensure_access(who, org_id)
            .map_err(|_| Error::<T>::PermissionDenied)?;
        ensure!(!org.suspended, Error::<T>::PermissionDenied);
        Ok(org)
    }

    /// Incerment certificate index
    pub fn increment_index() -> u64 {
        let next_id = <CertIdIndex<T>>::try_get().unwrap_or(0).saturating_add(1);
        <CertIdIndex<T>>::put(next_id);
        next_id
    }

    /// Generate hash for randomly generated certificate identification.
    pub fn generate_hash(data: Vec<u8>) -> CertId {
        let mut hash: [u8; 32] = Default::default();
        hash.copy_from_slice(&T::Hashing::hash(&data).encode()[..32]);
        hash
    }

    /// Generate hash for randomly generated certificate identification.
    /// 
    /// Issue ID ini merupakan hash dari data yang
    /// kemudian di-truncate untuk agar pendek
    /// dengan cara hanya mengambil 5 chars dari awal dan akhir
    /// dari hash dalam bentuk base58
    pub fn generate_issued_id(data: Vec<u8>) -> IssuedId {
        let hash = T::Hashing::hash(&data).encode().to_base58();
        let first = hash.chars().take(5);
        let last = hash.chars().skip(hash.len() - 5);
        first
            .into_iter()
            .chain(last)
            .collect::<String>()
            .as_bytes()
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as pallet_certificate;

    use frame_support::{
        assert_noop, assert_ok, ord_parameter_types, parameter_types, traits::Time,
    };
    use frame_system::EnsureSignedBy;
    use sp_core::{sr25519, H256};
    use sp_runtime::{
        testing::Header,
        traits::{BadOrigin, BlakeTwo256, IdentityLookup},
    };

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Module, Call, Storage},
            Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
            Did: pallet_did::{Module, Call, Storage, Event<T>},
            Organization: pallet_organization::{Module, Call, Storage, Event<T>},
            Certificate: pallet_certificate::{Module, Call, Storage, Event<T>},
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub BlockWeights: frame_system::limits::BlockWeights =
            frame_system::limits::BlockWeights::simple_max(1024);
    }
    impl frame_system::Config for Test {
        type BaseCallFilter = ();
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Call = Call;
        type Hashing = BlakeTwo256;
        type AccountId = sr25519::Public;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = Event;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<u64>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
    }
    parameter_types! {
        pub const ExistentialDeposit: u64 = 1;
    }
    impl pallet_balances::Config for Test {
        type MaxLocks = ();
        type Balance = u64;
        type Event = Event;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
    }
    parameter_types! {
        pub const MinOrgNameLength: usize = 3;
        pub const MaxOrgNameLength: usize = 100;
        pub const MaxMemberCount: usize = 100;
        pub const CreationFee: u64 = 20;
    }
    // ord_parameter_types! {
    //     pub const One: u64 = 1;
    // }
    parameter_types! {
        pub const MinimumPeriod: u64 = 5;
    }
    impl pallet_timestamp::Config for Test {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
        type WeightInfo = ();
    }

    impl pallet_did::Config for Test {
        type Event = Event;
        type Public = sr25519::Public;
        type Signature = sr25519::Signature;
        type Time = Timestamp;
        type WeightInfo = pallet_did::weights::SubstrateWeight<Self>;
    }

    // use pallet_organization::tests::{ALICE, BOB, CHARLIE, DAVE, EVE};

    ord_parameter_types! {
        pub const Root: sr25519::Public = sp_keyring::Sr25519Keyring::Alice.public();
    }

    impl pallet_organization::Config for Test {
        type Event = Event;
        type CreationFee = CreationFee;
        type Currency = Balances;
        type Payment = ();
        type ForceOrigin = EnsureSignedBy<Root, sr25519::Public>;
        type MinOrgNameLength = MinOrgNameLength;
        type MaxOrgNameLength = MaxOrgNameLength;
        type MaxMemberCount = MaxMemberCount;
        type WeightInfo = pallet_organization::weights::SubstrateWeight<Self>;
    }

    impl Config for Test {
        type Event = Event;
        type ForceOrigin = EnsureSignedBy<Root, sr25519::Public>;
        type Time = Self;
        type CreatorOrigin = pallet_organization::EnsureOrgAdmin<Self>;
        type WeightInfo = ();
    }

    impl Time for Test {
        type Moment = u64;
        fn now() -> Self::Moment {
            let start = std::time::SystemTime::now();
            let since_epoch = start
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            since_epoch.as_millis() as u64
        }
    }

    type CertEvent = pallet_certificate::Event<Test>;

    fn last_event() -> CertEvent {
        System::events()
            .into_iter()
            .map(|r| r.event)
            .filter_map(|e| {
                if let Event::pallet_certificate(inner) = e {
                    Some(inner)
                } else {
                    None
                }
            })
            .last()
            .expect("Event expected")
    }

    // fn expect_event<E: Into<Event>>(e: E) {
    //     assert_eq!(last_event(), e.into());
    // }

    use sp_keyring::Sr25519Keyring::{Alice, Bob, Charlie, Dave, Eve};

    fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![(Alice.into(), 50), (Bob.into(), 10), (Charlie.into(), 20)],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        t.into()
    }

    macro_rules! create_org {
        ($name:literal, $to:expr) => {
            assert_ok!(Organization::create_org(
                Origin::signed(Alice.public()),
                $name.to_vec(),
                b"".to_vec(),
                $to,
                b"".to_vec(),
                b"".to_vec()
            ));
        };
    }

    fn get_last_created_cert_id() -> Option<CertId> {
        match last_event() {
            CertEvent::CertAdded(_, cert_id, _) => Some(cert_id),
            _ => None,
        }
    }

    fn get_last_issued_cert_id() -> Option<IssuedId> {
        match last_event() {
            CertEvent::CertIssued(cert_id, _) => Some(cert_id),
            _ => None,
        }
    }

    fn last_org_id() -> <Test as frame_system::Config>::AccountId {
        System::events()
            .into_iter()
            .map(|r| r.event)
            .filter_map(|ev| {
                if let Event::pallet_organization(
                    pallet_organization::Event::<Test>::OrganizationAdded(org_id, _),
                ) = ev
                {
                    Some(org_id)
                } else {
                    None
                }
            })
            .last()
            .expect("Org id expected")
    }

    #[test]
    fn issue_cert_should_work() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            create_org!(b"ORG1", Bob.into());

            let org_id = last_org_id();

            assert_ok!(Certificate::create_cert(
                Origin::signed(Bob.into()),
                org_id,
                b"CERT1".to_vec(),
                b"CERT1 desc".to_vec()
            ));

            let cert_id = get_last_created_cert_id().expect("cert_id of new created cert");
            println!("cert_id: {:#?}", cert_id.to_base58());
            assert_eq!(Certificate::get(&cert_id).map(|a| a.org_id), Some(org_id));

            System::set_block_number(2);

            assert_ok!(Certificate::issue_cert(
                Origin::signed(Bob.into()),
                org_id,
                cert_id,
                b"Dave".to_vec(),
                b"ADDITIONAL DATA".to_vec(),
                None
            ));
            let issued_id = get_last_issued_cert_id().expect("get last issued id");
            println!("issued_id: {:?}", std::str::from_utf8(&issued_id));
        });
    }

    #[test]
    fn only_org_admin_can_create_cert() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            create_org!(b"ORG2", Charlie.into());
            assert_noop!(
                Certificate::create_cert(
                    Origin::signed(Bob.into()),
                    last_org_id(),
                    b"CERT1".to_vec(),
                    b"CERT1 desc".to_vec()
                ),
                BadOrigin
            );
        });
    }
}
