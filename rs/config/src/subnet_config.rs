//! This module implements support for static configurations for components that
//! can be different for different subnet types.

use crate::execution_environment::SUBNET_HEAP_DELTA_CAPACITY;
use ic_base_types::NumBytes;
use ic_registry_subnet_type::SubnetType;
use ic_types::{Cycles, NumInstructions};

// We assume 1 cycles unit ≅ 1 CPU cycle, so on a 2 GHz CPU one message has
// approximately 2.5 seconds to be processed.
pub(crate) const MAX_INSTRUCTIONS_PER_MESSAGE: NumInstructions =
    NumInstructions::new((1 << 30) * 5);

/// The per subnet type configuration for the scheduler component
#[derive(Clone)]
pub struct SchedulerConfig {
    /// Number of canisters that the scheduler is allowed to schedule in
    /// parallel.
    pub scheduler_cores: usize,

    /// Maximum amount of instructions a single round can consume (on one
    /// thread).
    pub max_instructions_per_round: NumInstructions,

    /// Maximum amount of instructions a single message's execution can consume.
    /// This should be significantly smaller than `max_instructions_per_round`.
    pub max_instructions_per_message: NumInstructions,

    /// This specifies the upper limit on how much heap delta all the canisters
    /// together on the subnet can produce in between checkpoints. This is a
    /// soft limit in the sense, that we will continue to execute canisters as
    /// long the current delta size is below this limit and stop if the current
    /// size is above this limit. Hence, it is possible that the actual usage of
    /// the subnet goes above this limit.
    pub subnet_heap_delta_capacity: NumBytes,
}

impl SchedulerConfig {
    pub fn application_subnet() -> Self {
        Self {
            // The gen 1 production machines should have 64 cores. We expect
            // that up to half might be needed for the IC protocol so we
            // allow the scheduler to use the other half for running
            // canisters in parallel.
            scheduler_cores: 32,

            subnet_heap_delta_capacity: SUBNET_HEAP_DELTA_CAPACITY,
            max_instructions_per_round: MAX_INSTRUCTIONS_PER_MESSAGE * 5,
            max_instructions_per_message: MAX_INSTRUCTIONS_PER_MESSAGE,
        }
    }

    pub fn system_subnet() -> Self {
        let max_instructions_per_message = NumInstructions::from(1_000_000_000_000u64);
        Self {
            // The gen 1 production machines should have 64 cores. We expect
            // that up to half might be needed for the IC protocol so we
            // allow the scheduler to use the other half for running
            // canisters in parallel.
            scheduler_cores: 32,

            subnet_heap_delta_capacity: SUBNET_HEAP_DELTA_CAPACITY,
            max_instructions_per_round: max_instructions_per_message * 5,
            max_instructions_per_message,
        }
    }

    pub fn verified_application_subnet() -> Self {
        Self {
            // The gen 1 production machines should have 64 cores. We expect
            // that up to half might be needed for the IC protocol so we
            // allow the scheduler to use the other half for running
            // canisters in parallel.
            scheduler_cores: 32,

            subnet_heap_delta_capacity: SUBNET_HEAP_DELTA_CAPACITY,
            max_instructions_per_round: MAX_INSTRUCTIONS_PER_MESSAGE * 5,
            max_instructions_per_message: MAX_INSTRUCTIONS_PER_MESSAGE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CyclesAccountManagerConfig {
    /// Fee for creating canisters on a subnet
    pub canister_creation_fee: Cycles,

    /// Fee for every update message executed
    pub update_message_execution_fee: Cycles,

    /// Fee for every 10 instructions executed when executing update type
    /// messages. The reason for using 10 and not 1 is so support fees of less
    /// than 1 cycles per instruction.
    pub ten_update_instructions_execution_fee: Cycles,

    /// Fee for every inter-canister call performed. This includes the fee for
    /// sending the request and receiving the response.
    pub xnet_call_fee: Cycles,

    /// Fee for every byte sent in an inter-canister call. The fee is for bytes
    /// sent in the request and response.
    pub xnet_byte_transmission_fee: Cycles,

    /// Fee for every ingress message received.
    pub ingress_message_reception_fee: Cycles,

    /// Fee for every byte received in an ingress message.
    pub ingress_byte_reception_fee: Cycles,

    /// Fee for storing a GiB of data per second.
    pub gib_storage_per_second_fee: Cycles,

    /// Fee for each percent of the reserved compute allocation. Note that
    /// reserved compute allocation is a scarce resource, and should be
    /// appropriately charged for.
    pub compute_percent_allocated_per_second_fee: Cycles,
}

impl CyclesAccountManagerConfig {
    pub fn application_subnet() -> Self {
        Self {
            // Setting the fee to create canisters to a significant amount to
            // discourage potential DoS attacks that involve creating a ton of
            // canisters on the network.
            canister_creation_fee: Cycles::new(1_000_000_000_000),
            compute_percent_allocated_per_second_fee: Cycles::new(100_000),

            // The following fields are set based on an experiment where we
            // measured how many resources a representative benchmark on a
            // representative subnet is using.
            update_message_execution_fee: Cycles::new(6_000_000),
            ten_update_instructions_execution_fee: Cycles::new(40),
            xnet_call_fee: Cycles::new(9_000_000),
            xnet_byte_transmission_fee: Cycles::new(70_000),
            ingress_message_reception_fee: Cycles::new(22_600_000),
            ingress_byte_reception_fee: Cycles::new(140_000),
            // 40 SDR per GiB per year => 40e12 Cycles per year
            gib_storage_per_second_fee: Cycles::new(1_270_000),
        }
    }

    pub fn verified_application_subnet() -> Self {
        Self {
            canister_creation_fee: Cycles::new(100_000_000_000),
            compute_percent_allocated_per_second_fee: Cycles::new(100_000),

            // The following fields are set based on a thought experiment where
            // we estimated how many resources a representative benchmark on a
            // verified subnet is using.
            update_message_execution_fee: Cycles::new(590_000),
            ten_update_instructions_execution_fee: Cycles::new(4),
            xnet_call_fee: Cycles::new(260_000),
            xnet_byte_transmission_fee: Cycles::new(1_000),
            ingress_message_reception_fee: Cycles::new(1_200_000),
            ingress_byte_reception_fee: Cycles::new(2_000),
            // 4 SDR per GiB per year => 4e12 Cycles per year
            gib_storage_per_second_fee: Cycles::new(127_000),
        }
    }

    /// All processing is free on system subnets
    pub fn system_subnet() -> Self {
        Self {
            canister_creation_fee: Cycles::new(0),
            compute_percent_allocated_per_second_fee: Cycles::new(0),
            update_message_execution_fee: Cycles::new(0),
            ten_update_instructions_execution_fee: Cycles::new(0),
            xnet_call_fee: Cycles::new(0),
            xnet_byte_transmission_fee: Cycles::new(0),
            ingress_message_reception_fee: Cycles::new(0),
            ingress_byte_reception_fee: Cycles::new(0),
            gib_storage_per_second_fee: Cycles::new(0),
        }
    }
}

/// The per subnet type configuration for CoW Memory Manager
#[derive(Clone)]
pub struct CowMemoryManagerConfig {
    /// Flag to enable or disable the feature
    pub enabled: bool,
}

impl CowMemoryManagerConfig {
    pub fn application_subnet() -> Self {
        Self { enabled: false }
    }

    pub fn system_subnet() -> Self {
        Self { enabled: false }
    }

    pub fn verified_application_subnet() -> Self {
        Self { enabled: false }
    }
}

/// If a component has at least one static configuration that is different for
/// different subnet types, then it is included in this struct.
#[derive(Clone)]
pub struct SubnetConfig {
    pub scheduler_config: SchedulerConfig,
    pub cycles_account_manager_config: CyclesAccountManagerConfig,
    pub cow_memory_manager_config: CowMemoryManagerConfig,
}

impl SubnetConfig {
    /// Returns the subnet configuration for the application subnet type.
    pub fn default_application_subnet() -> Self {
        Self {
            scheduler_config: SchedulerConfig::application_subnet(),
            cycles_account_manager_config: CyclesAccountManagerConfig::application_subnet(),
            cow_memory_manager_config: CowMemoryManagerConfig::application_subnet(),
        }
    }

    /// Returns the subnet configuration for the system subnet type.
    pub fn default_system_subnet() -> Self {
        Self {
            scheduler_config: SchedulerConfig::system_subnet(),
            cycles_account_manager_config: CyclesAccountManagerConfig::system_subnet(),
            cow_memory_manager_config: CowMemoryManagerConfig::system_subnet(),
        }
    }

    /// Returns the subnet configuration for the verified application subnet
    /// type.
    pub fn default_verified_application_subnet() -> Self {
        Self {
            scheduler_config: SchedulerConfig::verified_application_subnet(),
            cycles_account_manager_config: CyclesAccountManagerConfig::verified_application_subnet(
            ),
            cow_memory_manager_config: CowMemoryManagerConfig::verified_application_subnet(),
        }
    }
}

/// A struct that holds the per subnet configuration for all the subnet types on
/// the internet computer.
pub struct SubnetConfigs {
    system_subnet: SubnetConfig,
    application_subnet: SubnetConfig,
    verified_application_subnet: SubnetConfig,
}

impl Default for SubnetConfigs {
    fn default() -> Self {
        Self {
            system_subnet: SubnetConfig::default_system_subnet(),
            application_subnet: SubnetConfig::default_application_subnet(),
            verified_application_subnet: SubnetConfig::default_verified_application_subnet(),
        }
    }
}

impl SubnetConfigs {
    /// Returns the appropriate subnet configuration based on the subnet type.
    pub fn own_subnet_config(&self, own_subnet_type: SubnetType) -> SubnetConfig {
        match own_subnet_type {
            SubnetType::Application => self.application_subnet.clone(),
            SubnetType::System => self.system_subnet.clone(),
            SubnetType::VerifiedApplication => self.verified_application_subnet.clone(),
        }
    }
}
