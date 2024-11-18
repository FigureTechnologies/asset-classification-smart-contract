searchState.loadedDescShard("asset_classification_smart_contract", 0, "Asset Classification Smart Contract\nThe entrypoint for all external commands sent to the …\nContains all types and base functionality used to …\nContains all execution routes used by the contract file.\nContains the functionality used in the contract file to …\nContains the functionality used in the contract file to …\nContains the functionality used in the contract file to …\nComplex structs used to perform intensive operations in a …\nMiscellaneous functionalities that do not logically belong …\nFunctionality used to ensure the logical integrity of …\nThe entry point used when an external address desires to …\nThe entry point used when an external address instantiates …\nThe entry point used when migrating a live contract …\nThe entry point used when an external address desires to …\nContains each error type emitted by the contract.\nContains each message taken as a request by the contract.\nContains the core internal storage functionalities for the …\nContains all structs used to drive core functionality …\nThis error is encountered when an asset is attempted in …\nAn error emitted when a verifier attempts to run the …\nThis error is encountered when the onboarding process …\nThis error is encountered when the onboarding process is …\nThis error indicates that an asset was attempted to be …\nAn interceptor for a Bech32 Error.\nA massive enum including all the different error scenarios …\nDenotes that an existing VerifierDetailV2 has the same …\nAn error that can be used in a circumstance where a named …\nIndicates that a bech32 address was provided that does not …\nAn error that can occur during a migration that indicates …\nAn error that can occur during a migration that indicates …\nA generic error that specifies that some form of provided …\nAn error emitted by the validation that indicates that …\nAn error emitted when an internal issue arises, indicating …\nAn error that indicates that a scope inspected during the …\nAn error that occurs when a lookup is attempted for a …\nAn error that occurs when a unique key is violated during …\nOccurs when a mandatory data lookup is performed on the …\nAn interceptor for a SemVer Error.\nAn interceptor for a Cosmwasm Error.\nA generic error that occurs when an address attempts to …\nAn error emitted when an account attempts to initiate the …\nThis error occurs when a SerializedEnum is received from a …\nAn error that can occur when the contract is in an …\nA placeholder error that can be used as a stopgap during …\nThis error is encountered when an asset type is attempted …\nThis error can occur when a target VerifierDetailV2 does …\nAn interceptor for a Uuid Error.\nReturns the argument unchanged.\nConstructs an instance of the GenericError variant, …\nCalls <code>U::from(self)</code>.\nThe invalid bech32 address.\nThe asset type for which onboarding has already been …\nThe asset type for which verification has already been …\nThe asset type for which verification is pending for this …\nThe type of asset that is currently disabled.\nThe asset type that was used to resolve the verifier\nThe type of asset that could not be located for onboarding.\nThe asset_type selected during onboarding.\nThe name of the existing contract.  Should correlate to …\nThe version of the contract currently active on the …\nDenotes the correct msg that was expected to be provided.\nThe bech32 address of the account that has been requested …\nA message further explaining the issue.\nA free-form text description of the reason that the scope …\nA message describing the reason that the resource could …\nA free-form text description of the reason that the record …\nA free-form text description of the record that could not …\nA free-form text description of why the action was not …\nA free-form text description of what went wrong in …\nA free-form text description of why the operation was …\nA collection of messages that indicate every issue present …\nIndicates the type of message that was sent, causing the …\nThe name of the incorrect contract.\nThe version of the stored code used in the migration.\nA free-form text description of the error that occurred.\nThe type value of the serialized enum that could not be …\nThe bech32 scope address of the already-onboarded asset.\nThe bech32 address of the scope that has already been …\nThe bech32 address of the scope that does not appear to …\nThe bech32 scope address of the asset that has been …\nThe bech32 address of the scope that is awaiting …\nThe current onboarding status in the AssetScopeAttribute …\nThe bech32 address of the verifier that will perform …\nThe bech32 address of the account that attempted to run …\nThe bech32 address of the target verifier.\n<strong>This route is only accessible to the contract’s admin </strong>…\n<strong>This route is only accessible to the contract’s admin </strong>…\nPerforms a standard migration using the underlying …\n<strong>This route is only accessible to the contract’s admin </strong>…\nDefines all routes in which the contract can be executed.  …\nThe struct used to instantiate the contract.  Utilized in …\nThe struct used to migrate the contract from one code …\nSub-level struct that defines optional changes that can …\nThis route is the primary interaction point for most …\nThis route can be used to retrieve a specific …\nThis route can be used to retrieve all AssetDefinitionV3s …\nThis route can be used to retrieve an existing …\nThis route can be used to retrieve a list of existing …\nThis route can be used to retrieve an existing …\nDefines all routes in which the contract can be queried.  …\nThis route can be used to retrieve the internal contract …\nThis route can be used to retrieve the internal contract …\n<strong>This route is only accessible to the contract’s admin </strong>…\n<strong>This route is only accessible to the contract’s admin </strong>…\n<strong>This route is only accessible to the contract’s admin </strong>…\n<strong>This route is only accessible to the contract’s admin </strong>…\nThis route is specifically designed to allow a Verifier …\nAll the initial AssetDefinitionV3s for the contract.  This …\nThe root name from which all asset names branch.  All …\nIf <code>true</code>, the contract will automatically try to bind its …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nNotes whether or not any options have been specified.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA boolean value allowing for less restrictions to be …\nSets the contract admin to a new address when populated.  …\nAn optional parameter that allows the specification of a …\nLike in the OnboardAsset message, this parameter allows …\nA vector of AccessRoute to be used instead of the existing …\nAn optional parameter that will cause the emitted events …\nAn asset definition input value defining all of the new …\nAn asset definition input value defining all of the …\nA name that must directly match one of the contract’s …\nThe asset type this verification result is for\nThe type of asset for which the definition’s enabled …\nThe type of asset for which the new VerifierDetailV2 will …\nThe type of asset for which the VerifierDetailV2 will be …\nThe asset type to update access routes for\nThe asset type to delete the definition for\nThe value of enabled after the toggle takes place.  This …\nExpects an AssetIdentifier-compatible SerializedEnum.\nExpects an AssetIdentifier-compatible SerializedEnum.\nExpects an AssetIdentifier-compatible SerializedEnum.\nAn optional string describing the result of the …\nCorresponds to the bech32 address of the account that …\nA boolean indicating whether or not verification was …\nThe new verifier detail to be added to the asset …\nThe updated verifier detail to be modified in the asset …\nThe bech32 address of a Verifier Account associated with …\nVarious optional values that dictate additional behavior …\nThe asset type to query for\nThe asset type to query for\nThe asset type to query for pending verification fee …\nExpects an AssetIdentifier-compatible SerializedEnum.\nExpects an AssetIdentifier-compatible SerializedEnum.\nExpects an AssetIdentifier-compatible SerializedEnum.\nStores the main configurations for the contract internally.\nThe Provenance Blockchain bech32 address that maintains …\nThe root name from which all asset names branch.  All …\nAttempts to delete an existing asset definition by asset …\nAttempts to delete an existing payment detail by scope …\nReturns the argument unchanged.\nInserts a new asset definition into storage. If a value …\nInserts a new payment detail into storage.  If a value …\nCalls <code>U::from(self)</code>.\nA boolean value allowing for less restrictions to be …\nFinds an existing asset definition by asset type, or …\nFinds an existing fee payment detail by scope address, or …\nFinds an existing asset definition in state by checking …\nAttempts to find an existing fee payment detail by scope …\nConstructs a new instance of this struct for the …\nReplaces an existing asset definition in state with the …\nDefines a collection of AccessRoute for a specific address.\nDefines a method of obtaining underlying asset data for a …\nDefines a specific asset type associated with the …\nAn enum containing interchangeable values that can be used …\nAn enum that denotes the various states that an …\nAn asset scope attribute contains all relevant information …\nA simple wrapper for the result of a verification for a …\nVarious fields describing an entity, which could be an …\nDefines an external account designated as a recipient of …\nDefines a stored set of values for charging fees to the …\nA node that defines how much onboarding should cost and …\nA simple struct that allows a type and value to be …\nDefines fees and values that can be used when …\nDefines the fees and addresses for a single verifier …\nDefines a collection of AccessRoute for a specific address.\nAllows access definitions to be differentiated based on …\nIndicates that the access definition was created by the …\nIndicates that the access definition was created by the …\nA collection of AccessRoute structs that define methods of …\nDefines the source that created this definition.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct, ensuring that …\nThe bech32 address of the account that created the …\nDefines a method of obtaining underlying asset data for a …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nAn optional name parameter, allowing the creator of the …\nConstructs a new instance of this struct.\nA path to a resource that can provide underlying asset …\nConstructs an instance of this struct with an explicitly …\nConstructs an instance of this struct, omitting the <code>name</code> …\nMoves the struct to a new instance of itself with all …\nAllows the user to optionally specify the enabled flag on …\nDefines a specific asset type associated with the …\nClones the values contained within this struct into an …\nThe unique name of the asset associated with the …\nThe name of the asset associated with the definition.  …\nHelper functionality to retrieve the base contract name …\nHelper functionality to use the base contract name from …\nWhether or not to bind a Provenance Blockchain Name Module …\nA pretty human-readable name for this asset type (vs a …\nA pretty human-readable name for this asset type (vs a …\nIndicates whether or not the asset definition is enabled …\nIndicates whether or not the asset definition is enabled …\nReturns the argument unchanged.\nReturns the argument unchanged.\nHelper functionality to retrieve a verifier detail from …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMoves this struct into an instance of AssetDefinitionV3\nConstructs a new instance of AssetDefinitionV3, setting …\nConstructs a new instance of this struct.\nConverts the asset_type value to lowercase and serializes …\nIndividual verifier definitions.  There can be many …\nIndividual verifier definitions.  There can be many …\nAn enum containing interchangeable values that can be used …\nA simple named collection of both the asset uuid and scope …\nA uuid v4 represented by a string.\nA bech32 Provenance Blockchain address that begins with “…\nCreates a new instance of this enum as the AssetUuid …\nA uuid v4 value.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConverts a SerializedEnum instance to one of the variants …\nFetches the asset uuid value from this enum.  The AssetUuid…\nFetches the scope address value from this enum.  The …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct with the provided …\nCreates a new instance of this enum as the ScopeAddress …\nA bech32 address value with an hrp of “scope”.\nTakes the value provided and derives both values from it, …\nConverts the specific variant of this enum to a …\nIndicates that the asset has been verified and has been …\nAn enum that denotes the various states that an …\nIndicates that the asset has been verified and is …\nIndicates that the asset has been onboarded but has yet to …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nAn asset scope attribute contains all relevant information …\nAll provided access definitions are stored in the …\nThe name of the type of asset that is being used to …\nA unique uuid v4 value that defines the asset contained …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nThe most recent verification is kept on the scope …\nConstructs a new instance of AssetScopeAttribute from the …\nIndicates the portion of the classification process at …\nThe bech32 address of the account that requested this …\nThe bech32 address with a prefix of “scope” that …\nThe bech32 address of the account that the requestor …\nA simple wrapper for the result of a verification for a …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nA free-form message describing the result of the …\nIf true, the asset is deemed as successfully classified.  …\nVarious fields describing an entity, which could be an …\nA short description of the entity’s purpose.\nReturns the argument unchanged.\nA web link that can send observers to the organization …\nCalls <code>U::from(self)</code>.\nA short name describing the entity.\nConstructs a new instance of this struct.\nA web link that can send observers to the source code of …\nDefines an external account designated as a recipient of …\nThe Provenance Blockchain bech32 address belonging to the …\nAn optional set of fields that define the fee destination, …\nThe amount to be distributed to this account from the …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nConstructs a new instance of this struct with an entity …\nDefines an individual fee to be charged to an account …\nDefines a fee established from a VerifierDetailV2 and its …\nThe amount to be charged during the asset verification …\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA name describing to the end user (requestor) the purpose …\nConstructs a new instance of this struct by deriving all …\nThe breakdown of each fee charge.  This vector will always …\nThe bech32 address of the recipient of the fee, derived …\nThe bech32 address of the onboarded scope related to the …\nDetermines the aggregate amount paid via all payments.\nConverts all the payments into Provenance Blockchain bank …\nDefines costs used to onboard an asset to the contract for …\nThe amount of coin to be paid when an asset is sent to the …\nAny specific fee destinations that should be sent to …\nReturns the argument unchanged.\nSums all the fee amounts held within the individual fee …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThere is a bug in cosmwasm 1.0.0’s interaction with …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nConverts this value to an instance of an asset identifier …\nSpecifies the type of enum to deserialize into. Maps into …\nSpecifies the string value to be used for the type.\nThe root subsequent classifications node for a …\nSpecifies the asset types that an asset can be to have the …\nThe onboarding cost to use when classifying an asset using …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nDefines the fees and addresses for a single verifier …\nThe Provenance Blockchain bech32 address of the verifier …\nAn optional set of fields that define the verifier, …\nEach account that should receive fees when onboarding a …\nReturns the argument unchanged.\nPacks the root-level onboarding_cost and fee_destinations …\nCalculates a sum of all held fee_destinations respective …\nDetermines the values to use for retrying classification …\nDetermines the values to use for classifying an asset that …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThe total amount charged to use the onboarding process of …\nThe coin denomination used for this onboarding process.\nDefines the cost to use in place of the root …\nAn optional set of fields that define behaviors when …\nContains the functionality used by the AddAssetDefinition …\nContains the functionality used by the AddAssetVerifier …\nContains the functionality used by the …\nContains the functionality used by the OnboardAsset …\nContains the functionality used by the …\nContains the functionality used by the UpdateAccessRoutes …\nContains the functionality used by the …\nContains the functionality used by the UpdateAssetVerifier …\nContains the functionality used by the VerifyAsset …\nA transformation of ExecuteMsg::AddAssetDefinition for …\nThe function used by execute when an …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nA transformation of ExecuteMsg::AddAssetVerifier for ease …\nThe function used by execute when an …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nA transformation of ExecuteMsg::DeleteAssetDefinition for …\nRoute implementation for ExecuteMsg::DeleteAssetDefinition.\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nA transformation of ExecuteMsg::OnboardAsset for ease of …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nThe function used by execute when an …\nA transformation of ExecuteMsg::ToggleAssetDefinition for …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThe function used by execute when an …\nA transformation of ExecuteMsg::UpdateAccessRoutes for …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThe function used by execute when an …\nA transformation of ExecuteMsg::UpdateAssetDefinition for …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThe function used by execute when an …\nA transformation of ExecuteMsg::UpdateAssetVerifier for …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nThe function used by execute when an …\nA transformation of ExecuteMsg::VerifyAsset for ease of …\nReturns the argument unchanged.\nAttempts to create an instance of this struct from a …\nCalls <code>U::from(self)</code>.\nThe function used by execute when an …\nThe main functionality executed when the smart contract is …\nThe main functionality executed when the smart contract is …\nThe main entrypoint function for running a code migration. …\nModule for structs and helper functions containing the …\nThe main entrypoint function for running a code migration. …\nAutomatically derived from the Cargo.toml’s name …\nAutomatically derived from the Cargo.toml’s version …\nHolds both the contract’s unique name and version. Used …\nThe name of the contract, set to the value of CONTRACT_NAME…\nReturns the argument unchanged.\nFetches, if possible, the current version information for …\nCalls <code>U::from(self)</code>.\nSets the version info for the given contract to the …\nSets the contract’s version definition directly to the …\nThe version of the contract, set to the value of …\nA query that fetches a target AssetDefinitionV3 from the …\nA query that fetches all AssetDefinitionV3s from the …\nA query that attempts to find all AssetScopeAttributes on …\nA query that attempts to find an AssetScopeAttribute for a …\nA query that attempts to find a FeePaymentDetail stored …\nA query that directly returns the contract’s stored …\nA query that directly returns the contract’s stored …\nA query that fetches a target AssetDefinitionV3 from the …\nA query that fetches all AssetDefinitionV3s from the …\nFetches an AssetScopeAttribute by the scope address value, …\nFetches a list of AssetScopeAttribute by the scope address …\nFetches an AssetScopeAttribute by either the asset uuid or …\nFetches an AssetScopeAttribute by the asset uuid value …\nFetches an AssetScopeAttribute by the scope address value …\nFetches an AssetScopeAttribute by the scope address value, …\nFetches an AssetScopeAttribute by the scope address value …\nFetches an AssetScopeAttribute by either the asset uuid or …\nFetches an AssetScopeAttribute by the asset uuid value …\nFetches an AssetScopeAttribute by the scope address value …\nA query that fetches a target FeePaymentDetail from the …\nA query that directly returns the contract’s stored …\nPulls the version info for the contract out of the version …\nDefines a trait used for fetching and interacting with …\nTies all service code together into a cohesive struct to …\nAllows dynamic delegation of a cosmwasm DepsMut to prevent …\nSpecifies a trait used for dynamically aggregating …\nA trait used for fetching and interacting with asset …\nAttempts to fetch all asset attributes currently attached …\nAttempts to fetch an attribute currently attached to a …\nDetermines if a scope exists with an AssetScopeAttribute …\nAttempts to generate the CosmosMsg values required to …\nAttempts to fetch all asset attributes currently attached …\nAttempts to fetch asset attributes currently attached to a …\nAlters the internal values of the AssetScopeAttribute …\nAttempts to generate the CosmosMsg values required to …\nTies all service code together into a cohesive struct to …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nAllows dynamic delegation of a cosmwasm DepsMut to prevent …\nMoves the held DepsMut back to the caller.\nFunctionally retrieves the result of a usage of the held …\nSpecifies a trait used for dynamically aggregating …\nMoves an existing message into the service’s collection …\nAppends any number of existing messages by reference to …\nDeletes all held messages from the service’s internal …\nRetrieves all messages that have been appended to the …\nDefines various types with type aliases to shorten syntax …\nDefines all global constant values used throughout the …\nFunctions that perform common actions for the execute, …\nAllows dynamic delegation of a cosmwasm DepsMut to prevent …\nHelpers to ensure that emitting event attributes on execute…\nMiscellaneous functions to use in various scenarios …\nUtility functions that facilitate interaction with …\nUtility functions for interacting with bech32 addresses in …\nGlobal traits to be used across various areas of the …\nA container that allows a struct to manage a Vec, mutating …\nAll contract pathways with exceptional code should return …\nShortens the lengthy response type for contract …\nContains the error value\nContains the error value\nContains the success value\nContains the success value\nValue = EventAdditionalMetadata meta string.\nValue = Event Type correlating to EvenType enum into …\nValue = Asset UUID (String).\nValue = Asset UUID (String).\nValue = Asset Type (String).\nValue = The new onboarding status of an AssetScopeAttribute…\nValue = Any new value being changed that can be coerced to …\nA constant declaration to ensure the word “nhash” does …\nValue = Scope ID Tied to the Asset (String).\nValue = The scope owner that sent the onboarding message.\nAll denominations of coin that are valid for a verifier …\nValue = The address of the verifier associated with the …\nCreates a message for charging a custom fee.\nEnsures that only the admin of the contract can call into …\nEnsures that the info provided to the route does not …\nHolds a ref cell to a DepsMut, which allows it to be …\nReturns the argument unchanged.\nRelinquishes the held DepsMut to the caller with a move.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of the DepsContainer.\nAllows the encapsulated DepsMut value to be used while the …\nOccurs when the contract is executed to add an asset …\nOccurs when the contract is executed to add an asset …\nOccurs when the contract is executed to delete an asset …\nA helper collection that allows underlying processes to …\nA helper struct to emit attributes for a Response.\nAn enum that contains all different event types that can …\nOccurs when the contract is instantiated with instantiate.\nOccurs when the contract is migrated with migrate.\nOccurs when the contract is executed to onboard an asset.\nOccurs when the contract is executed to toggle an asset …\nOccurs when the contract is executed to update access …\nOccurs when the contract is executed to update an asset …\nOccurs when the contract is executed to update an asset …\nOccurs when the contract is executed to verify an asset.\nAppends a new key and value pair to the internal fields …\nUtilizes the implementation of Into to automatically …\nCertain events like onboard_asset require a standard set …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nAggregates and deterministically sorts the internal …\nReturns <code>true</code> only if metadata fields have been added with …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConstructs a new instance of this struct.\nConstructs a new instance of this struct with an empty …\nAppends a dynamic set of additional metadata to an …\nAppends an asset type value to an existing EventAttributes …\nAppends an onboarding status value to an existing …\nAppends a dynamic value to an existing EventAttributes and …\nAppends a scope address bech32 value to an existing …\nAppends a scope owner bech32 value to an existing …\nAppends a verifier address bech32 value to an existing …\nA helper to form a message for adding an attribute Adapted …\nA helper to form a message for adding a JSON attribute. …\nCreates a message that sends funds of the specified …\nDetermines how many elements within the provided reference …\nTrims down a vector of AccessRoute to ensure that the …\nConverts an asset type and a contract base name into an …\nConverts an asset type and scope address into a grant id …\nGenerates a name bind message that will properly assign …\nTakes an existing vector, moves it into this function, …\nAttempts to convert a CosmosMsg into a …\nAttempts to convert a CosmosMsg into a MsgBindNameRequest\nAttempts to convert a CosmosMsg into a …\nAttempts to convert a CosmosMsg into a …\nA helper to form a message for updating an existing …\nA helper that ensures address params are non-empty. Copied …\nA helper that ensures string params are non-empty. Copied …\nHelper function to generate an “add attribute” …\nConverts a string containing an asset uuid into a scope …\nValidates that the address is valid by decoding to base …\nTakes a string representation of a scope address and …\nAllows any Sized type to functionally move itself into an …\nContains a RefCell that manages a Vec of …\nAppends an owned, mutable instance of a Vec containing …\nRemoves all values from the inner Vec.\nReturns the argument unchanged.\nFetches the actual value inside the RefCell, moving the …\nFetches a cloned set of the owned values. Useful for early …\nFetches a copied set of the owned values.  Copying is more …\nCalls <code>U::from(self)</code>.\nConstruct a new instance of a container, starting with an …\nPushes a single owned item to the contained Vec.\nThe inner values of the container that are manipulated by …\nValidates the integrity of an intercepted ExecuteMsg …\nValidates the integrity of an intercepted InitMsg and its …\nThe main branch of validation for an execute msg.  Funnels …\nValidates that an asset definition value is properly …\nValidates that an asset definition input value is properly …\nValidates the integrity of an intercepted InitMsg and its …\nValidates that a verifier detail is properly formed, …\nValidates that a verifier detail is properly formed, …")