use std::{collections::HashMap, sync::OnceLock};

use crate::{Error, Result};

static DB_FILE_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn get_db_file_map() -> &'static HashMap<&'static str, &'static str> {
    DB_FILE_MAP.get_or_init(|| {
        let mut m = HashMap::new();

        m.insert("accountstorecategory", "AccountStoreCategory.dbd");
        m.insert("accountstoreitem", "AccountStoreItem.dbd");
        m.insert("achievement", "Achievement.dbd");
        m.insert("achievement_category", "Achievement_Category.dbd");
        m.insert("achievement_criteria", "Achievement_Criteria.dbd");
        m.insert("actionbargroup", "ActionBarGroup.dbd");
        m.insert("actionbargroupentry", "ActionBarGroupEntry.dbd");
        m.insert("adventurejournal", "AdventureJournal.dbd");
        m.insert("adventurejournalitem", "AdventureJournalItem.dbd");
        m.insert("adventuremappoi", "AdventureMapPOI.dbd");
        m.insert("alliedrace", "AlliedRace.dbd");
        m.insert("alliedraceracialability", "AlliedRaceRacialAbility.dbd");
        m.insert("altminimap", "AltMinimap.dbd");
        m.insert("altminimapfiledata", "AltMinimapFiledata.dbd");
        m.insert("altminimapwmo", "AltMinimapWMO.dbd");
        m.insert("animacable", "AnimaCable.dbd");
        m.insert("animamaterial", "AnimaMaterial.dbd");
        m.insert("animationdata", "AnimationData.dbd");
        m.insert("animationnames", "AnimationNames.dbd");
        m.insert("animkit", "AnimKit.dbd");
        m.insert("animkitboneset", "AnimKitBoneSet.dbd");
        m.insert("animkitbonesetalias", "AnimKitBoneSetAlias.dbd");
        m.insert("animkitconfig", "AnimKitConfig.dbd");
        m.insert("animkitconfigboneset", "AnimKitConfigBoneSet.dbd");
        m.insert("animkitpriority", "AnimKitPriority.dbd");
        m.insert("animkitreplacement", "AnimKitReplacement.dbd");
        m.insert("animkitsegment", "AnimKitSegment.dbd");
        m.insert("animreplacement", "AnimReplacement.dbd");
        m.insert("animreplacementset", "AnimReplacementSet.dbd");
        m.insert("aoibox", "AoiBox.dbd");
        m.insert("areaassignment", "AreaAssignment.dbd");
        m.insert("areaconditionaldata", "AreaConditionalData.dbd");
        m.insert("areagroup", "AreaGroup.dbd");
        m.insert("areagroupmember", "AreaGroupMember.dbd");
        m.insert("areamidiambiences", "AreaMIDIAmbiences.dbd");
        m.insert("areapoi", "AreaPOI.dbd");
        m.insert("areapoisortedworldstate", "AreaPOISortedWorldState.dbd");
        m.insert("areapoistate", "AreaPOIState.dbd");
        m.insert("areapoiuiwidgetset", "AreaPOIUiWidgetSet.dbd");
        m.insert("areatable", "AreaTable.dbd");
        m.insert("areatrigger", "AreaTrigger.dbd");
        m.insert("areatriggeractionset", "AreaTriggerActionSet.dbd");
        m.insert("areatriggerboundedplane", "AreaTriggerBoundedPlane.dbd");
        m.insert("areatriggerbox", "AreaTriggerBox.dbd");
        m.insert(
            "areatriggercreateproperties",
            "AreaTriggerCreateProperties.dbd",
        );
        m.insert("areatriggercylinder", "AreaTriggerCylinder.dbd");
        m.insert("areatriggerdisk", "AreaTriggerDisk.dbd");
        m.insert("areatriggersphere", "AreaTriggerSphere.dbd");
        m.insert("arenatrackeditem", "ArenaTrackedItem.dbd");
        m.insert("armorlocation", "ArmorLocation.dbd");
        m.insert("artifact", "Artifact.dbd");
        m.insert("artifactappearance", "ArtifactAppearance.dbd");
        m.insert("artifactappearanceset", "ArtifactAppearanceSet.dbd");
        m.insert("artifactcategory", "ArtifactCategory.dbd");
        m.insert("artifactitemtotransmog", "ArtifactItemToTransmog.dbd");
        m.insert("artifactpower", "ArtifactPower.dbd");
        m.insert("artifactpowerlink", "ArtifactPowerLink.dbd");
        m.insert("artifactpowerpicker", "ArtifactPowerPicker.dbd");
        m.insert("artifactpowerrank", "ArtifactPowerRank.dbd");
        m.insert("artifactquestxp", "ArtifactQuestXP.dbd");
        m.insert("artifacttier", "ArtifactTier.dbd");
        m.insert("artifactunlock", "ArtifactUnlock.dbd");
        m.insert("assistedcombat", "AssistedCombat.dbd");
        m.insert("animacylinder", "AnimaCylinder.dbd");
        m.insert("areafarclipoverride", "AreaFarClipOverride.dbd");
        m.insert("arenaccitem", "ArenaCcItem.dbd");
        m.insert("assistedcombatrule", "AssistedCombatRule.dbd");
        m.insert("azeritetierunlockset", "AzeriteTierUnlockSet.dbd");
        m.insert("battlepetdisplayoverride", "BattlePetDisplayOverride.dbd");
        m.insert("broadcasttextduration", "BroadcastTextDuration.dbd");
        m.insert("cfg_realms", "Cfg_Realms.dbd");
        m.insert(
            "charcomponenttexturesections",
            "CharComponentTextureSections.dbd",
        );
        m.insert("chrclassracesex", "ChrClassRaceSex.dbd");
        m.insert(
            "chrcustomizationconversion",
            "ChrCustomizationConversion.dbd",
        );
        m.insert("chrproficiency", "ChrProficiency.dbd");
        m.insert("clientsettings", "ClientSettings.dbd");
        m.insert("communityicon", "CommunityIcon.dbd");
        m.insert("assistedcombatstep", "AssistedCombatStep.dbd");
        m.insert("attackanimkits", "AttackAnimKits.dbd");
        m.insert("attackanimtypes", "AttackAnimTypes.dbd");
        m.insert("auctionhouse", "AuctionHouse.dbd");
        m.insert("auctionhousecategory", "AuctionHouseCategory.dbd");
        m.insert("azeriteempowereditem", "AzeriteEmpoweredItem.dbd");
        m.insert("azeriteessence", "AzeriteEssence.dbd");
        m.insert("azeriteessencepower", "AzeriteEssencePower.dbd");
        m.insert("azeriteitem", "AzeriteItem.dbd");
        m.insert("azeriteitemmilestonepower", "AzeriteItemMilestonePower.dbd");
        m.insert(
            "azeriteknowledgemultiplier",
            "AzeriteKnowledgeMultiplier.dbd",
        );
        m.insert("azeritelevelinfo", "AzeriteLevelInfo.dbd");
        m.insert("azeritepower", "AzeritePower.dbd");
        m.insert("azeritepowersetmember", "AzeritePowerSetMember.dbd");
        m.insert("azeritetierunlock", "AzeriteTierUnlock.dbd");
        m.insert("azeriteunlockmapping", "AzeriteUnlockMapping.dbd");
        m.insert("bankbagslotprices", "BankBagSlotPrices.dbd");
        m.insert("banktab", "BankTab.dbd");
        m.insert("bannedaddons", "BannedAddons.dbd");
        m.insert("barbershopstyle", "BarberShopStyle.dbd");
        m.insert("barrageeffect", "BarrageEffect.dbd");
        m.insert("battlemasterlist", "BattlemasterList.dbd");
        m.insert("battlemasterlistxmap", "BattlemasterListXMap.dbd");
        m.insert("battlepaycurrency", "BattlepayCurrency.dbd");
        m.insert("battlepetability", "BattlePetAbility.dbd");
        m.insert("battlepetabilityeffect", "BattlePetAbilityEffect.dbd");
        m.insert("battlepetabilitystate", "BattlePetAbilityState.dbd");
        m.insert("battlepetabilityturn", "BattlePetAbilityTurn.dbd");
        m.insert("battlepetbreedquality", "BattlePetBreedQuality.dbd");
        m.insert("battlepetbreedstate", "BattlePetBreedState.dbd");
        m.insert("battlepeteffectproperties", "BattlePetEffectProperties.dbd");
        m.insert("battlepetnpcteammember", "BattlePetNPCTeamMember.dbd");
        m.insert("battlepetspecies", "BattlePetSpecies.dbd");
        m.insert("battlepetspeciesstate", "BattlePetSpeciesState.dbd");
        m.insert("battlepetspeciesxability", "BattlePetSpeciesXAbility.dbd");
        m.insert("battlepetspeciesxcovenant", "BattlePetSpeciesXCovenant.dbd");
        m.insert("battlepetstate", "BattlePetState.dbd");
        m.insert("battlepetvisual", "BattlePetVisual.dbd");
        m.insert("beameffect", "BeamEffect.dbd");
        m.insert("beckontrigger", "BeckonTrigger.dbd");
        m.insert("bonewindmodifiermodel", "BoneWindModifierModel.dbd");
        m.insert("bonewindmodifiers", "BoneWindModifiers.dbd");
        m.insert("bonusroll", "BonusRoll.dbd");
        m.insert("bounty", "Bounty.dbd");
        m.insert("bountyset", "BountySet.dbd");
        m.insert("broadcasttext", "BroadcastText.dbd");
        m.insert("broadcasttextsoundstate", "BroadcastTextSoundState.dbd");
        m.insert("broadcasttextvostate", "BroadcastTextVOState.dbd");
        m.insert("cameraeffect", "CameraEffect.dbd");
        m.insert("cameraeffectentry", "CameraEffectEntry.dbd");
        m.insert("cameramode", "CameraMode.dbd");
        m.insert("camerashakes", "CameraShakes.dbd");
        m.insert("campaign", "Campaign.dbd");
        m.insert("campaignxcondition", "CampaignXCondition.dbd");
        m.insert("campaignxquestline", "CampaignXQuestLine.dbd");
        m.insert("castableraidbuffs", "CastableRaidBuffs.dbd");
        m.insert("celestialbody", "CelestialBody.dbd");
        m.insert("cfg_categories", "Cfg_Categories.dbd");
        m.insert("cfg_configs", "Cfg_Configs.dbd");
        m.insert("cfg_gamerules", "Cfg_GameRules.dbd");
        m.insert("cfg_languages", "Cfg_Languages.dbd");
        m.insert("cfg_regions", "Cfg_Regions.dbd");
        m.insert("cfg_timeeventregiongroup", "Cfg_TimeEventRegionGroup.dbd");
        m.insert(
            "challengemodeitembonusoverride",
            "ChallengeModeItemBonusOverride.dbd",
        );
        m.insert("challengemodereward", "ChallengeModeReward.dbd");
        m.insert("challengemodexreward", "ChallengeModeXReward.dbd");
        m.insert("charactercreatecameras", "CharacterCreateCameras.dbd");
        m.insert("characterfaceboneset", "CharacterFaceBoneSet.dbd");
        m.insert("characterfacialhairstyles", "CharacterFacialHairStyles.dbd");
        m.insert("characterloadout", "CharacterLoadout.dbd");
        m.insert("characterloadoutitem", "CharacterLoadoutItem.dbd");
        m.insert("characterloadoutpet", "CharacterLoadoutPet.dbd");
        m.insert("characterserviceinfo", "CharacterServiceInfo.dbd");
        m.insert("charbaseinfo", "CharBaseInfo.dbd");
        m.insert("charbasesection", "CharBaseSection.dbd");
        m.insert(
            "charcomponenttexturelayouts",
            "CharComponentTextureLayouts.dbd",
        );
        m.insert("charhairgeosets", "CharHairGeosets.dbd");
        m.insert("charhairtextures", "CharHairTextures.dbd");
        m.insert("charsectioncondition", "CharSectionCondition.dbd");
        m.insert("charsections", "CharSections.dbd");
        m.insert("charshipment", "CharShipment.dbd");
        m.insert("charshipmentcontainer", "CharShipmentContainer.dbd");
        m.insert("charstartkit", "CharStartKit.dbd");
        m.insert("charstartoutfit", "CharStartOutfit.dbd");
        m.insert("chartexturevariationsv2", "CharTextureVariationsV2.dbd");
        m.insert("chartitles", "CharTitles.dbd");
        m.insert("charvariations", "CharVariations.dbd");
        m.insert("chatchannels", "ChatChannels.dbd");
        m.insert("chatprofanity", "ChatProfanity.dbd");
        m.insert("chrclasses", "ChrClasses.dbd");
        m.insert("chrclassesxpowertypes", "ChrClassesXPowerTypes.dbd");
        m.insert("chrclasstitle", "ChrClassTitle.dbd");
        m.insert("chrclassuichrmodelinfo", "ChrClassUIChrModelInfo.dbd");
        m.insert("chrclassuidisplay", "ChrClassUIDisplay.dbd");
        m.insert("chrclassvillain", "ChrClassVillain.dbd");
        m.insert("chrcreateclassanimtarget", "ChrCreateClassAnimTarget.dbd");
        m.insert(
            "chrcreateclassanimtargetinfo",
            "ChrCreateClassAnimTargetInfo.dbd",
        );
        m.insert(
            "chrcustclientchoiceconversion",
            "ChrCustClientChoiceConversion.dbd",
        );
        m.insert("chrcustgeocomponentlink", "ChrCustGeoComponentLink.dbd");
        m.insert("chrcustitemgeomodify", "ChrCustItemGeoModify.dbd");
        m.insert("chrcustomization", "ChrCustomization.dbd");
        m.insert("chrcustomizationboneset", "ChrCustomizationBoneSet.dbd");
        m.insert("chrcustomizationcategory", "ChrCustomizationCategory.dbd");
        m.insert("chrcustomizationchoice", "ChrCustomizationChoice.dbd");
        m.insert("chrcustomizationcondmodel", "ChrCustomizationCondModel.dbd");
        m.insert(
            "chrcustomizationdisplayinfo",
            "ChrCustomizationDisplayInfo.dbd",
        );
        m.insert("chrcustomizationelement", "ChrCustomizationElement.dbd");
        m.insert("chrcustomizationgeoset", "ChrCustomizationGeoset.dbd");
        m.insert("chrcustomizationglyphpet", "ChrCustomizationGlyphPet.dbd");
        m.insert("chrcustomizationmaterial", "ChrCustomizationMaterial.dbd");
        m.insert("chrcustomizationoption", "ChrCustomizationOption.dbd");
        m.insert("chrcustomizationreq", "ChrCustomizationReq.dbd");
        m.insert("chrcustomizationreqchoice", "ChrCustomizationReqChoice.dbd");
        m.insert(
            "chrcustomizationskinnedmodel",
            "ChrCustomizationSkinnedModel.dbd",
        );
        m.insert("chrcustomizationvisreq", "ChrCustomizationVisReq.dbd");
        m.insert("chrcustomizationvoice", "ChrCustomizationVoice.dbd");
        m.insert("chrmodel", "ChrModel.dbd");
        m.insert("chrmodelmaterial", "ChrModelMaterial.dbd");
        m.insert("chrmodeltexturelayer", "ChrModelTextureLayer.dbd");
        m.insert("chrmodeltexturetarget", "ChrModelTextureTarget.dbd");
        m.insert("chrraceracialability", "ChrRaceRacialAbility.dbd");
        m.insert("chrraces", "ChrRaces.dbd");
        m.insert("chrracescreatescreenicon", "ChrRacesCreateScreenIcon.dbd");
        m.insert("chrracesping", "ChrRacesPing.dbd");
        m.insert("chrracexchrmodel", "ChrRaceXChrModel.dbd");
        m.insert("chrselectbackgroundcdi", "ChrSelectBackgroundCDI.dbd");
        m.insert("chrspecialization", "ChrSpecialization.dbd");
        m.insert("chrupgradebucket", "ChrUpgradeBucket.dbd");
        m.insert("chrupgradebucketspell", "ChrUpgradeBucketSpell.dbd");
        m.insert("chrupgradetier", "ChrUpgradeTier.dbd");
        m.insert("cinematic", "Cinematic.dbd");
        m.insert("cinematiccamera", "CinematicCamera.dbd");
        m.insert("cinematicsequences", "CinematicSequences.dbd");
        m.insert("cinematicsubtitle", "CinematicSubtitle.dbd");
        m.insert("clientsceneeffect", "ClientSceneEffect.dbd");
        m.insert("cloakdampening", "CloakDampening.dbd");
        m.insert("cloneeffect", "CloneEffect.dbd");
        m.insert(
            "collectablesourceencounter",
            "CollectableSourceEncounter.dbd",
        );
        m.insert(
            "collectablesourceencountersparse",
            "CollectableSourceEncounterSparse.dbd",
        );
        m.insert("collectablesourceinfo", "CollectableSourceInfo.dbd");
        m.insert("collectablesourcequest", "CollectableSourceQuest.dbd");
        m.insert(
            "collectablesourcequestsparse",
            "CollectableSourceQuestSparse.dbd",
        );
        m.insert("collectablesourcevendor", "CollectableSourceVendor.dbd");
        m.insert(
            "collectablesourcevendorsparse",
            "CollectableSourceVendorSparse.dbd",
        );
        m.insert("colorbanding", "ColorBanding.dbd");
        m.insert("combatcondition", "CombatCondition.dbd");
        m.insert("commentatorindirectspell", "CommentatorIndirectSpell.dbd");
        m.insert("commentatorstartlocation", "CommentatorStartLocation.dbd");
        m.insert(
            "commentatortrackedcooldown",
            "CommentatorTrackedCooldown.dbd",
        );
        m.insert("componentmodelfiledata", "ComponentModelFileData.dbd");
        m.insert("componenttexturefiledata", "ComponentTextureFileData.dbd");
        m.insert("conditionalchrmodel", "ConditionalChrModel.dbd");
        m.insert("conditionalcontenttuning", "ConditionalContentTuning.dbd");
        m.insert(
            "conditionalcreaturemodeldata",
            "ConditionalCreatureModelData.dbd",
        );
        m.insert("conditionalitemappearance", "ConditionalItemAppearance.dbd");
        m.insert("configurationwarning", "ConfigurationWarning.dbd");
        m.insert("consolescripts", "ConsoleScripts.dbd");
        m.insert("contentpush", "ContentPush.dbd");
        m.insert("contentrestrictionrule", "ContentRestrictionRule.dbd");
        m.insert("contentrestrictionruleset", "ContentRestrictionRuleSet.dbd");
        m.insert("contenttuning", "ContentTuning.dbd");
        m.insert("contenttuningdescription", "ContentTuningDescription.dbd");
        m.insert("contenttuningxdifficulty", "ContentTuningXDifficulty.dbd");
        m.insert("contenttuningxexpected", "ContentTuningXExpected.dbd");
        m.insert("contenttuningxlabel", "ContentTuningXLabel.dbd");
        m.insert("contribution", "Contribution.dbd");
        m.insert("contributionstyle", "ContributionStyle.dbd");
        m.insert(
            "contributionstylecontainer",
            "ContributionStyleContainer.dbd",
        );
        m.insert("conversationline", "ConversationLine.dbd");
        m.insert("cooldownset", "CooldownSet.dbd");
        m.insert("cooldownsetlinkedspell", "CooldownSetLinkedSpell.dbd");
        m.insert("cooldownsetspell", "CooldownSetSpell.dbd");
        m.insert("corruptioneffects", "CorruptionEffects.dbd");
        m.insert("covenant", "Covenant.dbd");
        m.insert("craftingdata", "CraftingData.dbd");
        m.insert(
            "craftingdataenchantquality",
            "CraftingDataEnchantQuality.dbd",
        );
        m.insert("craftingdataitemquality", "CraftingDataItemQuality.dbd");
        m.insert("craftingdifficulty", "CraftingDifficulty.dbd");
        m.insert("craftingdifficultyquality", "CraftingDifficultyQuality.dbd");
        m.insert("craftingitemquality", "CraftingItemQuality.dbd");
        m.insert("craftingorder", "CraftingOrder.dbd");
        m.insert("craftingorderhouse", "CraftingOrderHouse.dbd");
        m.insert("craftingorderxlabel", "CraftingOrderXLabel.dbd");
        m.insert("craftingquality", "CraftingQuality.dbd");
        m.insert("craftingreagenteffect", "CraftingReagentEffect.dbd");
        m.insert("craftingreagentquality", "CraftingReagentQuality.dbd");
        m.insert(
            "craftingreagentrequirement",
            "CraftingReagentRequirement.dbd",
        );
        m.insert("creature", "Creature.dbd");
        m.insert("creaturedifficulty", "CreatureDifficulty.dbd");
        m.insert(
            "creaturedifficultytreasure",
            "CreatureDifficultyTreasure.dbd",
        );
        m.insert("creaturedisplayinfo", "CreatureDisplayInfo.dbd");
        m.insert("creaturedisplayinfocond", "CreatureDisplayInfoCond.dbd");
        m.insert(
            "creaturedisplayinfocondxchoice",
            "CreatureDisplayInfoCondXChoice.dbd",
        );
        m.insert("creaturedisplayinfoevt", "CreatureDisplayInfoEvt.dbd");
        m.insert(
            "creaturedisplayinfogeosetdata",
            "CreatureDisplayInfoGeosetData.dbd",
        );
        m.insert("creaturedisplayinfooption", "CreatureDisplayInfoOption.dbd");
        m.insert("creaturedisplayinfotrn", "CreatureDisplayInfoTrn.dbd");
        m.insert(
            "creaturedisplayxuimodelscene",
            "CreatureDisplayXUIModelScene.dbd",
        );
        m.insert("creaturedispxuicamera", "CreatureDispXUiCamera.dbd");
        m.insert("creaturefamily", "CreatureFamily.dbd");
        m.insert(
            "creaturefamilyxuimodelscene",
            "CreatureFamilyXUIModelScene.dbd",
        );
        m.insert("creatureimmunities", "CreatureImmunities.dbd");
        m.insert("creaturelabel", "CreatureLabel.dbd");
        m.insert("creaturemodeldata", "CreatureModelData.dbd");
        m.insert("creaturemovementinfo", "CreatureMovementInfo.dbd");
        m.insert("creaturesounddata", "CreatureSoundData.dbd");
        m.insert("creaturesoundfidget", "CreatureSoundFidget.dbd");
        m.insert("creaturespelldata", "CreatureSpellData.dbd");
        m.insert("creaturetype", "CreatureType.dbd");
        m.insert("creaturexdisplayinfo", "CreatureXDisplayInfo.dbd");
        m.insert("creaturexuiwidgetset", "CreatureXUiWidgetSet.dbd");
        m.insert("criteria", "Criteria.dbd");
        m.insert("criteriatree", "CriteriaTree.dbd");
        m.insert("criteriatreexeffect", "CriteriaTreeXEffect.dbd");
        m.insert("currencycategory", "CurrencyCategory.dbd");
        m.insert("currencycontainer", "CurrencyContainer.dbd");
        m.insert("currencysource", "CurrencySource.dbd");
        m.insert("currencytypes", "CurrencyTypes.dbd");
        m.insert("curve", "Curve.dbd");
        m.insert("curvepoint", "CurvePoint.dbd");
        m.insert("dancemoves", "DanceMoves.dbd");
        m.insert("deaththudlookups", "DeathThudLookups.dbd");
        m.insert("decalproperties", "DecalProperties.dbd");
        m.insert("declinedword", "DeclinedWord.dbd");
        m.insert("declinedwordcases", "DeclinedWordCases.dbd");
        m.insert("delvesseasonxspell", "DelvesSeasonXSpell.dbd");
        m.insert("destructiblemodeldata", "DestructibleModelData.dbd");
        m.insert("deviceblacklist", "DeviceBlacklist.dbd");
        m.insert("devicedefaultsettings", "DeviceDefaultSettings.dbd");
        m.insert("difficulty", "Difficulty.dbd");
        m.insert("displayseason", "DisplaySeason.dbd");
        m.insert("dissolveeffect", "DissolveEffect.dbd");
        m.insert("drivecapability", "DriveCapability.dbd");
        m.insert("drivecapabilitytier", "DriveCapabilityTier.dbd");
        m.insert("driverblacklist", "DriverBlacklist.dbd");
        m.insert("dungeonencounter", "DungeonEncounter.dbd");
        m.insert("dungeonmap", "DungeonMap.dbd");
        m.insert("dungeonmapchunk", "DungeonMapChunk.dbd");
        m.insert("durabilitycosts", "DurabilityCosts.dbd");
        m.insert("durabilityquality", "DurabilityQuality.dbd");
        m.insert("edgegloweffect", "EdgeGlowEffect.dbd");
        m.insert("emotes", "Emotes.dbd");
        m.insert("emotestext", "EmotesText.dbd");
        m.insert("emotestextdata", "EmotesTextData.dbd");
        m.insert("emotestextsound", "EmotesTextSound.dbd");
        m.insert("enumeratedstring", "EnumeratedString.dbd");
        m.insert("environmentaldamage", "EnvironmentalDamage.dbd");
        m.insert("exhaustion", "Exhaustion.dbd");
        m.insert("expectedstat", "ExpectedStat.dbd");
        m.insert("expectedstatmod", "ExpectedStatMod.dbd");
        m.insert("extraabilityinfo", "ExtraAbilityInfo.dbd");
        m.insert("faction", "Faction.dbd");
        m.insert("factiongroup", "FactionGroup.dbd");
        m.insert("factiontemplate", "FactionTemplate.dbd");
        m.insert("filedata", "FileData.dbd");
        m.insert("filedatacomplete", "FileDataComplete.dbd");
        m.insert("filepaths", "FilePaths.dbd");
        m.insert(
            "flightcapabilityxglideevent",
            "FlightCapabilityXGlideEvent.dbd",
        );
        m.insert("footprinttextures", "FootprintTextures.dbd");
        m.insert("footstepterrainlookup", "FootstepTerrainLookup.dbd");
        m.insert("friendshiprepreaction", "FriendshipRepReaction.dbd");
        m.insert("friendshipreputation", "FriendshipReputation.dbd");
        m.insert("fullscreeneffect", "FullScreenEffect.dbd");
        m.insert("gameclockdebug", "GameClockDebug.dbd");
        m.insert("gamemode", "GameMode.dbd");
        m.insert("gameobjectanimgroupmember", "GameObjectAnimGroupMember.dbd");
        m.insert("gameobjectartkit", "GameObjectArtKit.dbd");
        m.insert("gameobjectdiffanimmap", "GameObjectDiffAnimMap.dbd");
        m.insert(
            "gameobjectdisplaycondition",
            "GameObjectDisplayCondition.dbd",
        );
        m.insert("gameobjectdisplayinfo", "GameObjectDisplayInfo.dbd");
        m.insert(
            "gameobjectdisplayinfoxsoundkit",
            "GameObjectDisplayInfoXSoundKit.dbd",
        );
        m.insert("gameobjectlabel", "GameObjectLabel.dbd");
        m.insert("gameobjectsclient", "GameObjectsClient.dbd");
        m.insert("gameparameter", "GameParameter.dbd");
        m.insert("gametables", "GameTables.dbd");
        m.insert("gametips", "GameTips.dbd");
        m.insert("garrability", "GarrAbility.dbd");
        m.insert("garrabilitycategory", "GarrAbilityCategory.dbd");
        m.insert("garrabilityeffect", "GarrAbilityEffect.dbd");
        m.insert("garrautocombatant", "GarrAutoCombatant.dbd");
        m.insert("garrautospell", "GarrAutoSpell.dbd");
        m.insert("garrautospelleffect", "GarrAutoSpellEffect.dbd");
        m.insert("garrbuilding", "GarrBuilding.dbd");
        m.insert("garrbuildingdoodadset", "GarrBuildingDoodadSet.dbd");
        m.insert("garrbuildingplotinst", "GarrBuildingPlotInst.dbd");
        m.insert("garrclassspec", "GarrClassSpec.dbd");
        m.insert("garrclassspecplayercond", "GarrClassSpecPlayerCond.dbd");
        m.insert(
            "garrencountersetxencounter",
            "GarrEncounterSetXEncounter.dbd",
        );
        m.insert("garrencounterxmechanic", "GarrEncounterXMechanic.dbd");
        m.insert("garrfamilyname", "GarrFamilyName.dbd");
        m.insert("garrfollitemset", "GarrFollItemSet.dbd");
        m.insert("garrfollitemsetmember", "GarrFollItemSetMember.dbd");
        m.insert("garrfollower", "GarrFollower.dbd");
        m.insert("garrfollowerlevelxp", "GarrFollowerLevelXP.dbd");
        m.insert("garrfollowerquality", "GarrFollowerQuality.dbd");
        m.insert("garrfollowersetxfollower", "GarrFollowerSetXFollower.dbd");
        m.insert("garrfollowertype", "GarrFollowerType.dbd");
        m.insert("garrfolloweruicreature", "GarrFollowerUICreature.dbd");
        m.insert("garrfollowerxability", "GarrFollowerXAbility.dbd");
        m.insert("garrfollsupportspell", "GarrFollSupportSpell.dbd");
        m.insert("garrgivenname", "GarrGivenName.dbd");
        m.insert("garritemlevelupgradedata", "GarrItemLevelUpgradeData.dbd");
        m.insert("garrmechanicsetxmechanic", "GarrMechanicSetXMechanic.dbd");
        m.insert("garrmechanictype", "GarrMechanicType.dbd");
        m.insert("garrmission", "GarrMission.dbd");
        m.insert("garrmissionreward", "GarrMissionReward.dbd");
        m.insert("garrmissionset", "GarrMissionSet.dbd");
        m.insert("garrmissiontexture", "GarrMissionTexture.dbd");
        m.insert("garrmissiontype", "GarrMissionType.dbd");
        m.insert("garrmissionxencounter", "GarrMissionXEncounter.dbd");
        m.insert("garrmissionxfollower", "GarrMissionXFollower.dbd");
        m.insert("garrmssnbonusability", "GarrMssnBonusAbility.dbd");
        m.insert("garrplot", "GarrPlot.dbd");
        m.insert("garrplotbuilding", "GarrPlotBuilding.dbd");
        m.insert("garrplotinstance", "GarrPlotInstance.dbd");
        m.insert("garrplotuicategory", "GarrPlotUICategory.dbd");
        m.insert("garrsitelevel", "GarrSiteLevel.dbd");
        m.insert("garrspecialization", "GarrSpecialization.dbd");
        m.insert("garrstring", "GarrString.dbd");
        m.insert("garrtalent", "GarrTalent.dbd");
        m.insert("garrtalentcost", "GarrTalentCost.dbd");
        m.insert("garrtalentmappoi", "GarrTalentMapPOI.dbd");
        m.insert("garrtalentrank", "GarrTalentRank.dbd");
        m.insert("garrtalentrankgroupentry", "GarrTalentRankGroupEntry.dbd");
        m.insert(
            "garrtalentrankgroupresearchmod",
            "GarrTalentRankGroupResearchMod.dbd",
        );
        m.insert("garrtalentresearch", "GarrTalentResearch.dbd");
        m.insert(
            "garrtalentsocketproperties",
            "GarrTalentSocketProperties.dbd",
        );
        m.insert("garrtalenttree", "GarrTalentTree.dbd");
        m.insert(
            "garrtaltreexgarrtalresearch",
            "GarrTalTreeXGarrTalResearch.dbd",
        );
        m.insert("garrtype", "GarrType.dbd");
        m.insert("garruianimclassinfo", "GarrUiAnimClassInfo.dbd");
        m.insert("garruianimraceinfo", "GarrUiAnimRaceInfo.dbd");
        m.insert("glideevent", "GlideEvent.dbd");
        m.insert("glideeventblendtimes", "GlideEventBlendTimes.dbd");
        m.insert("globalcolor", "GlobalColor.dbd");
        m.insert("globalcurve", "GlobalCurve.dbd");
        m.insert("globalgamecontenttuning", "GlobalGameContentTuning.dbd");
        m.insert("globalplayercondition", "GlobalPlayerCondition.dbd");
        m.insert("globalplayerconditionset", "GlobalPlayerConditionSet.dbd");
        m.insert("globalstrings", "GlobalStrings.dbd");
        m.insert(
            "globaltable_playercondition",
            "GlobalTable_PlayerCondition.dbd",
        );
        m.insert("gluescreenemote", "GlueScreenEmote.dbd");
        m.insert("glyphbindablespell", "GlyphBindableSpell.dbd");
        m.insert("glyphexclusivecategory", "GlyphExclusiveCategory.dbd");
        m.insert("glyphproperties", "GlyphProperties.dbd");
        m.insert("glyphrequiredspec", "GlyphRequiredSpec.dbd");
        m.insert("glyphslot", "GlyphSlot.dbd");
        m.insert("gmsurveycurrentsurvey", "GMSurveyCurrentSurvey.dbd");
        m.insert("gmsurveyquestions", "GMSurveyQuestions.dbd");
        m.insert("gmsurveysurveys", "GMSurveySurveys.dbd");
        m.insert("gmticketcategory", "GMTicketCategory.dbd");
        m.insert("gossipnpcoption", "GossipNPCOption.dbd");
        m.insert(
            "gossipnpcoptiondisplayinfo",
            "GossipNPCOptionDisplayInfo.dbd",
        );
        m.insert("gossipoptionxuiwidgetset", "GossipOptionXUIWidgetSet.dbd");
        m.insert(
            "gossipuidisplayinfocondition",
            "GossipUIDisplayInfoCondition.dbd",
        );
        m.insert("gossipxgarrtalenttrees", "GossipXGarrTalentTrees.dbd");
        m.insert("gossipxuidisplayinfo", "GossipXUIDisplayInfo.dbd");
        m.insert("gradienteffect", "GradientEffect.dbd");
        m.insert("groundeffectdoodad", "GroundEffectDoodad.dbd");
        m.insert("groundeffecttexture", "GroundEffectTexture.dbd");
        m.insert("groupfinderactivity", "GroupFinderActivity.dbd");
        m.insert("groupfinderactivitygrp", "GroupFinderActivityGrp.dbd");
        m.insert("groupfindercategory", "GroupFinderCategory.dbd");
        m.insert("gtarmormitigationbylvl", "gtArmorMitigationByLvl.dbd");
        m.insert("gtbarbershopcostbase", "gtBarberShopCostBase.dbd");
        m.insert("gtbattlepettypedamagemod", "gtBattlePetTypeDamageMod.dbd");
        m.insert("gtbattlepetxp", "gtBattlePetXP.dbd");
        m.insert("gtchancetomeleecrit", "gtChanceToMeleeCrit.dbd");
        m.insert("gtchancetomeleecritbase", "gtChanceToMeleeCritBase.dbd");
        m.insert("gtchancetospellcrit", "gtChanceToSpellCrit.dbd");
        m.insert("gtchancetospellcritbase", "gtChanceToSpellCritBase.dbd");
        m.insert("gtcombatratings", "gtCombatRatings.dbd");
        m.insert("gtitemsocketcostperlevel", "gtItemSocketCostPerLevel.dbd");
        m.insert("gtmasterymultipliers", "gtMasteryMultipliers.dbd");
        m.insert("gtnpcmanacostscaler", "gtNPCManaCostScaler.dbd");
        m.insert("gtoctbasehpbyclass", "gtOCTBaseHPByClass.dbd");
        m.insert("gtoctbasempbyclass", "gtOCTBaseMPByClass.dbd");
        m.insert(
            "gtoctclasscombatratingscalar",
            "gtOCTClassCombatRatingScalar.dbd",
        );
        m.insert("gtocthpperstamina", "gtOCTHpPerStamina.dbd");
        m.insert("gtoctlevelexperience", "gtOCTLevelExperience.dbd");
        m.insert("gtoctregenhp", "gtOCTRegenHP.dbd");
        m.insert("gtoctregenmp", "gtOCTRegenMP.dbd");
        m.insert("gtregenhpperspt", "gtRegenHPPerSpt.dbd");
        m.insert("gtregenmpperspt", "gtRegenMPPerSpt.dbd");
        m.insert("gtresiliencedr", "gtResilienceDR.dbd");
        m.insert("gtspellscaling", "gtSpellScaling.dbd");
        m.insert("holidaynames", "HolidayNames.dbd");
        m.insert("holidays", "Holidays.dbd");
        m.insert("holidayxtimeevent", "HolidayXTimeEvent.dbd");
        m.insert("hotfix", "Hotfix.dbd");
        m.insert("hotfixes", "Hotfixes.dbd");
        m.insert("housedecor", "HouseDecor.dbd");
        m.insert("importpricearmor", "ImportPriceArmor.dbd");
        m.insert("importpricequality", "ImportPriceQuality.dbd");
        m.insert("importpriceshield", "ImportPriceShield.dbd");
        m.insert("importpriceweapon", "ImportPriceWeapon.dbd");
        m.insert("invasionclientdata", "InvasionClientData.dbd");
        m.insert("item-sparse", "Item-sparse.dbd");
        m.insert("item", "Item.dbd");
        m.insert("itemappearance", "ItemAppearance.dbd");
        m.insert("itemappearancexuicamera", "ItemAppearanceXUiCamera.dbd");
        m.insert("itemarmorquality", "ItemArmorQuality.dbd");
        m.insert("itemarmortotal", "ItemArmorTotal.dbd");
        m.insert("itembagfamily", "ItemBagFamily.dbd");
        m.insert("itembonus", "ItemBonus.dbd");
        m.insert("itembonuslist", "ItemBonusList.dbd");
        m.insert("itembonuslistgroup", "ItemBonusListGroup.dbd");
        m.insert("itembonuslistgroupentry", "ItemBonusListGroupEntry.dbd");
        m.insert("itembonuslistleveldelta", "ItemBonusListLevelDelta.dbd");
        m.insert(
            "itembonuslistwarforgeleveldelta",
            "ItemBonusListWarforgeLevelDelta.dbd",
        );
        m.insert("itembonusseason", "ItemBonusSeason.dbd");
        m.insert(
            "itembonusseasonbonuslistgroup",
            "ItemBonusSeasonBonusListGroup.dbd",
        );
        m.insert(
            "itembonusseasonupgradecost",
            "ItemBonusSeasonUpgradeCost.dbd",
        );
        m.insert("itembonussequencespell", "ItemBonusSequenceSpell.dbd");
        m.insert("itembonustree", "ItemBonusTree.dbd");
        m.insert("itembonustreegroupentry", "ItemBonusTreeGroupEntry.dbd");
        m.insert("itembonustreenode", "ItemBonusTreeNode.dbd");
        m.insert("craftingitem", "CraftingItem.dbd");
        m.insert("creaturedisplayinfoextra", "CreatureDisplayInfoExtra.dbd");
        m.insert("creaturexcontribution", "CreatureXContribution.dbd");
        m.insert("delvesseason", "DelvesSeason.dbd");
        m.insert("emoteanims", "EmoteAnims.dbd");
        m.insert("flightcapability", "FlightCapability.dbd");
        m.insert("gameobjects", "GameObjects.dbd");
        m.insert("garrencounter", "GarrEncounter.dbd");
        m.insert("garrmechanic", "GarrMechanic.dbd");
        m.insert("garrsitelevelplotinst", "GarrSiteLevelPlotInst.dbd");
        m.insert("gemproperties", "GemProperties.dbd");
        m.insert("gmsurveyanswers", "GMSurveyAnswers.dbd");
        m.insert(
            "groupfinderactivityxpvpbracket",
            "GroupFinderActivityXPvpBracket.dbd",
        );
        m.insert("guildcolorbackground", "GuildColorBackground.dbd");
        m.insert("holidaydescriptions", "HolidayDescriptions.dbd");
        m.insert("itemclass", "ItemClass.dbd");
        m.insert("itemcondextcosts", "ItemCondExtCosts.dbd");
        m.insert("itemcondition", "ItemCondition.dbd");
        m.insert("itemcontextpickerentry", "ItemContextPickerEntry.dbd");
        m.insert("itemconversion", "ItemConversion.dbd");
        m.insert("itemconversionentry", "ItemConversionEntry.dbd");
        m.insert("itemcreationcontext", "ItemCreationContext.dbd");
        m.insert("itemcreationcontextgroup", "ItemCreationContextGroup.dbd");
        m.insert("itemcurrencycost", "ItemCurrencyCost.dbd");
        m.insert("itemcurrencyvalue", "ItemCurrencyValue.dbd");
        m.insert("itemdamageammo", "ItemDamageAmmo.dbd");
        m.insert("itemdamageonehand", "ItemDamageOneHand.dbd");
        m.insert("itemdamageonehandcaster", "ItemDamageOneHandCaster.dbd");
        m.insert("itemdamageranged", "ItemDamageRanged.dbd");
        m.insert("itemdamagethrown", "ItemDamageThrown.dbd");
        m.insert("itemdamagetwohandcaster", "ItemDamageTwoHandCaster.dbd");
        m.insert("itemdamagewand", "ItemDamageWand.dbd");
        m.insert("itemdisenchantloot", "ItemDisenchantLoot.dbd");
        m.insert("itemdisplayinfo", "ItemDisplayInfo.dbd");
        m.insert(
            "itemdisplayinfomaterialres",
            "ItemDisplayInfoMaterialRes.dbd",
        );
        m.insert(
            "itemdisplayinfomodelmatres",
            "ItemDisplayInfoModelMatRes.dbd",
        );
        m.insert("itemdisplayxuicamera", "ItemDisplayXUiCamera.dbd");
        m.insert("itemeffect", "ItemEffect.dbd");
        m.insert("itemextendedcost", "ItemExtendedCost.dbd");
        m.insert("itemfallbackvisual", "ItemFallbackVisual.dbd");
        m.insert("itemfixup", "ItemFixup.dbd");
        m.insert("itemfixupaction", "ItemFixupAction.dbd");
        m.insert("itemgroupilvlscalingentry", "ItemGroupIlvlScalingEntry.dbd");
        m.insert("itemgroupsounds", "ItemGroupSounds.dbd");
        m.insert("itemlevelselector", "ItemLevelSelector.dbd");
        m.insert(
            "itemlevelselectorqualityset",
            "ItemLevelSelectorQualitySet.dbd",
        );
        m.insert("itemlevelwatermark", "ItemLevelWatermark.dbd");
        m.insert("itemlimitcategory", "ItemLimitCategory.dbd");
        m.insert(
            "itemlimitcategorycondition",
            "ItemLimitCategoryCondition.dbd",
        );
        m.insert("itemlogicalcost", "ItemLogicalCost.dbd");
        m.insert("itemlogicalcostgroup", "ItemLogicalCostGroup.dbd");
        m.insert("itemmodifiedappearance", "ItemModifiedAppearance.dbd");
        m.insert(
            "itemmodifiedappearanceextra",
            "ItemModifiedAppearanceExtra.dbd",
        );
        m.insert("itemnamedescription", "ItemNameDescription.dbd");
        m.insert("itemnameslotoverride", "ItemNameSlotOverride.dbd");
        m.insert("itemoffsetcurve", "ItemOffsetCurve.dbd");
        m.insert("itempetfood", "ItemPetFood.dbd");
        m.insert("itempricebase", "ItemPriceBase.dbd");
        m.insert("itempurchasegroup", "ItemPurchaseGroup.dbd");
        m.insert("itemrandomproperties", "ItemRandomProperties.dbd");
        m.insert("itemrandomsuffix", "ItemRandomSuffix.dbd");
        m.insert("itemrecraft", "ItemRecraft.dbd");
        m.insert("itemreforge", "ItemReforge.dbd");
        m.insert("itemsalvage", "ItemSalvage.dbd");
        m.insert("itemsalvageloot", "ItemSalvageLoot.dbd");
        m.insert("itemscalingconfig", "ItemScalingConfig.dbd");
        m.insert("itemsearchname", "ItemSearchName.dbd");
        m.insert("itemset", "ItemSet.dbd");
        m.insert("itemsetspell", "ItemSetSpell.dbd");
        m.insert("itemsparse", "ItemSparse.dbd");
        m.insert("itemspec", "ItemSpec.dbd");
        m.insert("itemspecoverride", "ItemSpecOverride.dbd");
        m.insert("itemsquishera", "ItemSquishEra.dbd");
        m.insert("itemsubclass", "ItemSubClass.dbd");
        m.insert("itemsubclassmask", "ItemSubClassMask.dbd");
        m.insert("itemtobattlepet", "ItemToBattlePet.dbd");
        m.insert("itemtomountspell", "ItemToMountSpell.dbd");
        m.insert("itemupgradepath", "ItemUpgradePath.dbd");
        m.insert("itemvisualeffects", "ItemVisualEffects.dbd");
        m.insert("itemvisuals", "ItemVisuals.dbd");
        m.insert("itemvisualsxeffect", "ItemVisualsXEffect.dbd");
        m.insert("itemxbonustree", "ItemXBonusTree.dbd");
        m.insert("itemxitemeffect", "ItemXItemEffect.dbd");
        m.insert("itemxtraitsystem", "ItemXTraitSystem.dbd");
        m.insert("journalencounter", "JournalEncounter.dbd");
        m.insert("journalencountercreature", "JournalEncounterCreature.dbd");
        m.insert("journalencounteritem", "JournalEncounterItem.dbd");
        m.insert("journalencountersection", "JournalEncounterSection.dbd");
        m.insert(
            "journalencounterxdifficulty",
            "JournalEncounterXDifficulty.dbd",
        );
        m.insert("journalencounterxmaploc", "JournalEncounterXMapLoc.dbd");
        m.insert("journalinstance", "JournalInstance.dbd");
        m.insert("journalinstanceentrance", "JournalInstanceEntrance.dbd");
        m.insert("journalitemxdifficulty", "JournalItemXDifficulty.dbd");
        m.insert("journalsectionxdifficulty", "JournalSectionXDifficulty.dbd");
        m.insert("journaltier", "JournalTier.dbd");
        m.insert("journaltierxinstance", "JournalTierXInstance.dbd");
        m.insert("keychain", "Keychain.dbd");
        m.insert("keystoneaffix", "KeystoneAffix.dbd");
        m.insert(
            "labelxcontentrestrictruleset",
            "LabelXContentRestrictRuleSet.dbd",
        );
        m.insert("languages", "Languages.dbd");
        m.insert("languagewords", "LanguageWords.dbd");
        m.insert("lfgdungeonexpansion", "LFGDungeonExpansion.dbd");
        m.insert("lfgdungeongroup", "LFGDungeonGroup.dbd");
        m.insert("lfgdungeons", "LFGDungeons.dbd");
        m.insert("lfgdungeonsgroupingmap", "LfgDungeonsGroupingMap.dbd");
        m.insert("lfgrolerequirement", "LFGRoleRequirement.dbd");
        m.insert("light", "Light.dbd");
        m.insert("lightdata", "LightData.dbd");
        m.insert("lightintband", "LightIntBand.dbd");
        m.insert("lightning", "Lightning.dbd");
        m.insert("lightparams", "LightParams.dbd");
        m.insert("lightparamslightshaft", "LightParamsLightShaft.dbd");
        m.insert("lightshaft", "LightShaft.dbd");
        m.insert("lightskybox", "LightSkybox.dbd");
        m.insert("lightworldshadow", "LightWorldShadow.dbd");
        m.insert("liquidmaterial", "LiquidMaterial.dbd");
        m.insert("liquidobject", "LiquidObject.dbd");
        m.insert("liquidtype", "LiquidType.dbd");
        m.insert("liquidtypextexture", "LiquidTypeXTexture.dbd");
        m.insert("livingworldobjecttemplate", "LivingWorldObjectTemplate.dbd");
        m.insert(
            "livingworldobjecttemplatemodel",
            "LivingWorldObjectTemplateModel.dbd",
        );
        m.insert("loadingscreens", "LoadingScreens.dbd");
        m.insert("loadingscreenskin", "LoadingScreenSkin.dbd");
        m.insert("loadingscreentaxisplines", "LoadingScreenTaxiSplines.dbd");
        m.insert("location", "Location.dbd");
        m.insert("lock", "Lock.dbd");
        m.insert("locktype", "LockType.dbd");
        m.insert("lookatcontroller", "LookAtController.dbd");
        m.insert("loretext", "LoreText.dbd");
        m.insert("loretextpublic", "LoreTextPublic.dbd");
        m.insert("mailtemplate", "MailTemplate.dbd");
        m.insert("managedworldstate", "ManagedWorldState.dbd");
        m.insert("managedworldstatebuff", "ManagedWorldStateBuff.dbd");
        m.insert("managedworldstateinput", "ManagedWorldStateInput.dbd");
        m.insert(
            "manifestinterfaceactionicon",
            "ManifestInterfaceActionIcon.dbd",
        );
        m.insert("manifestinterfacedata", "ManifestInterfaceData.dbd");
        m.insert("manifestinterfaceitemicon", "ManifestInterfaceItemIcon.dbd");
        m.insert("manifestinterfacetocdata", "ManifestInterfaceTOCData.dbd");
        m.insert("manifestmp3", "ManifestMP3.dbd");
        m.insert("map", "Map.dbd");
        m.insert("mapchallengemode", "MapChallengeMode.dbd");
        m.insert(
            "mapchallengemodeaffixcriteria",
            "MapChallengeModeAffixCriteria.dbd",
        );
        m.insert("mapdifficulty", "MapDifficulty.dbd");
        m.insert("mapdifficultyredirect", "MapDifficultyRedirect.dbd");
        m.insert("mapdifficultyxcondition", "MapDifficultyXCondition.dbd");
        m.insert("maploadingscreen", "MapLoadingScreen.dbd");
        m.insert("maprenderscale", "MapRenderScale.dbd");
        m.insert(
            "marketingpromotionsxlocale",
            "MarketingPromotionsXLocale.dbd",
        );
        m.insert("material", "Material.dbd");
        m.insert("mawpower", "MawPower.dbd");
        m.insert("mawpowerrarity", "MawPowerRarity.dbd");
        m.insert("mcrslotxmcrcategory", "MCRSlotXMCRCategory.dbd");
        m.insert("minortalent", "MinorTalent.dbd");
        m.insert("missiletargeting", "MissileTargeting.dbd");
        m.insert("mobilestrings", "MobileStrings.dbd");
        m.insert("modelfiledata", "ModelFileData.dbd");
        m.insert("modelmanifest", "ModelManifest.dbd");
        m.insert("modelnametomanifest", "ModelNameToManifest.dbd");
        m.insert("modelribbonquality", "ModelRibbonQuality.dbd");
        m.insert("modelsound", "ModelSound.dbd");
        m.insert("modelsoundanimentry", "ModelSoundAnimEntry.dbd");
        m.insert("modelsoundentry", "ModelSoundEntry.dbd");
        m.insert("modelsoundoverride", "ModelSoundOverride.dbd");
        m.insert("modelsoundoverridename", "ModelSoundOverrideName.dbd");
        m.insert("modelsoundsettings", "ModelSoundSettings.dbd");
        m.insert("modelsoundtagentry", "ModelSoundTagEntry.dbd");
        m.insert("modifiedcraftingcategory", "ModifiedCraftingCategory.dbd");
        m.insert("modifiedcraftingitem", "ModifiedCraftingItem.dbd");
        m.insert(
            "modifiedcraftingreagentitem",
            "ModifiedCraftingReagentItem.dbd",
        );
        m.insert(
            "modifiedcraftingreagentslot",
            "ModifiedCraftingReagentSlot.dbd",
        );
        m.insert("modifiedreagentitem", "ModifiedReagentItem.dbd");
        m.insert("modifiertree", "ModifierTree.dbd");
        m.insert("mount", "Mount.dbd");
        m.insert("mountcapability", "MountCapability.dbd");
        m.insert("mountequipment", "MountEquipment.dbd");
        m.insert("mounttype", "MountType.dbd");
        m.insert("mounttypexcapability", "MountTypeXCapability.dbd");
        m.insert("mountxdisplay", "MountXDisplay.dbd");
        m.insert(
            "mountxspellvisualkitpicker",
            "MountXSpellVisualKitPicker.dbd",
        );
        m.insert("movie", "Movie.dbd");
        m.insert("moviefiledata", "MovieFileData.dbd");
        m.insert("movieoverlays", "MovieOverlays.dbd");
        m.insert("movievariation", "MovieVariation.dbd");
        m.insert("multistateproperties", "MultiStateProperties.dbd");
        m.insert("multitransitionproperties", "MultiTransitionProperties.dbd");
        m.insert("mythicplusseason", "MythicPlusSeason.dbd");
        m.insert("mythicplusseasonkeyfloor", "MythicPlusSeasonKeyFloor.dbd");
        m.insert(
            "mythicplusseasonrewardlevels",
            "MythicPlusSeasonRewardLevels.dbd",
        );
        m.insert(
            "mythicplusseasontrackedaffix",
            "MythicPlusSeasonTrackedAffix.dbd",
        );
        m.insert(
            "mythicplusseasontrackedmap",
            "MythicPlusSeasonTrackedMap.dbd",
        );
        m.insert("namegen", "NameGen.dbd");
        m.insert("namesprofanity", "NamesProfanity.dbd");
        m.insert("namesreserved", "NamesReserved.dbd");
        m.insert("namesreservedlocale", "NamesReservedLocale.dbd");
        m.insert("npccraftingordercustomer", "NPCCraftingOrderCustomer.dbd");
        m.insert(
            "npccraftingordercustomerxlabel",
            "NPCCraftingOrderCustomerXLabel.dbd",
        );
        m.insert("npccraftingorderset", "NPCCraftingOrderSet.dbd");
        m.insert(
            "npccraftingordersetxcraftorder",
            "NPCCraftingOrderSetXCraftOrder.dbd",
        );
        m.insert(
            "npccraftingordersetxcustomer",
            "NPCCraftingOrderSetXCustomer.dbd",
        );
        m.insert(
            "npccraftingordersetxtreasure",
            "NPCCraftingOrderSetXTreasure.dbd",
        );
        m.insert("npcsounds", "NPCSounds.dbd");
        m.insert("numtalentsatlevel", "NumTalentsAtLevel.dbd");
        m.insert("objecteffect", "ObjectEffect.dbd");
        m.insert("objecteffectgroup", "ObjectEffectGroup.dbd");
        m.insert("objecteffectmodifier", "ObjectEffectModifier.dbd");
        m.insert("objecteffectpackage", "ObjectEffectPackage.dbd");
        m.insert("objecteffectpackageelem", "ObjectEffectPackageElem.dbd");
        m.insert("objecteffectstatename", "ObjectEffectStateName.dbd");
        m.insert("occluder", "Occluder.dbd");
        m.insert("occludercurtain", "OccluderCurtain.dbd");
        m.insert("occluderlocation", "OccluderLocation.dbd");
        m.insert("occludernode", "OccluderNode.dbd");
        m.insert("outlineeffect", "OutlineEffect.dbd");
        m.insert("overridespelldata", "OverrideSpellData.dbd");
        m.insert("package", "Package.dbd");
        m.insert("pagetextmaterial", "PageTextMaterial.dbd");
        m.insert("paragonreputation", "ParagonReputation.dbd");
        m.insert("particlecolor", "ParticleColor.dbd");
        m.insert("particulate", "Particulate.dbd");
        m.insert("particulatesound", "ParticulateSound.dbd");
        m.insert("path", "Path.dbd");
        m.insert("pathedge", "PathEdge.dbd");
        m.insert("pathnode", "PathNode.dbd");
        m.insert("pathnodeproperty", "PathNodeProperty.dbd");
        m.insert("pathproperty", "PathProperty.dbd");
        m.insert("perksactivity", "PerksActivity.dbd");
        m.insert("perksactivitycondition", "PerksActivityCondition.dbd");
        m.insert("perksactivitytag", "PerksActivityTag.dbd");
        m.insert("perksactivitythreshold", "PerksActivityThreshold.dbd");
        m.insert(
            "perksactivitythresholdgroup",
            "PerksActivityThresholdGroup.dbd",
        );
        m.insert("perksactivityxholidays", "PerksActivityXHolidays.dbd");
        m.insert("perksactivityxinterval", "PerksActivityXInterval.dbd");
        m.insert("perksuitheme", "PerksUITheme.dbd");
        m.insert("perksvendorcategory", "PerksVendorCategory.dbd");
        m.insert("perksvendoritem", "PerksVendorItem.dbd");
        m.insert("perksvendoritemuigroup", "PerksVendorItemUIGroup.dbd");
        m.insert("perksvendoritemuiinfo", "PerksVendorItemUIInfo.dbd");
        m.insert("perksvendoritemxinterval", "PerksVendorItemXInterval.dbd");
        m.insert("petitiontype", "PetitionType.dbd");
        m.insert("petloyalty", "PetLoyalty.dbd");
        m.insert("petpersonality", "PetPersonality.dbd");
        m.insert("phase", "Phase.dbd");
        m.insert("phaseshiftzonesounds", "PhaseShiftZoneSounds.dbd");
        m.insert("phasexphasegroup", "PhaseXPhaseGroup.dbd");
        m.insert("pingtype", "PingType.dbd");
        m.insert("playercompanioninfo", "PlayerCompanionInfo.dbd");
        m.insert("playercondition", "PlayerCondition.dbd");
        m.insert("itemchildequipment", "ItemChildEquipment.dbd");
        m.insert("itemdamagetwohand", "ItemDamageTwoHand.dbd");
        m.insert("itemlevelselectorquality", "ItemLevelSelectorQuality.dbd");
        m.insert("itemrangeddisplayinfo", "ItemRangedDisplayInfo.dbd");
        m.insert("itemupgrade", "ItemUpgrade.dbd");
        m.insert("journalinstancequeueloc", "JournalInstanceQueueLoc.dbd");
        m.insert("lightfloatband", "LightFloatBand.dbd");
        m.insert("locale", "Locale.dbd");
        m.insert("mapcelestialbody", "MapCelestialBody.dbd");
        m.insert("modelanimcloakdampening", "ModelAnimCloakDampening.dbd");
        m.insert("modifiedcraftingspellslot", "ModifiedCraftingSpellSlot.dbd");
        m.insert("musicoverride", "MusicOverride.dbd");
        m.insert(
            "npcmodelitemslotdisplayinfo",
            "NPCModelItemSlotDisplayInfo.dbd",
        );
        m.insert("paperdollitemframe", "PaperDollItemFrame.dbd");
        m.insert(
            "playerdataelementcharacter",
            "PlayerDataElementCharacter.dbd",
        );
        m.insert("playerdataflagaccount", "PlayerDataFlagAccount.dbd");
        m.insert("playerdataflagcharacter", "PlayerDataFlagCharacter.dbd");
        m.insert("playerinteractioninfo", "PlayerInteractionInfo.dbd");
        m.insert("pointlightconditionmap", "PointLightConditionMap.dbd");
        m.insert("positioner", "Positioner.dbd");
        m.insert("positionerstate", "PositionerState.dbd");
        m.insert("positionerstateentry", "PositionerStateEntry.dbd");
        m.insert("powerdisplay", "PowerDisplay.dbd");
        m.insert("powertype", "PowerType.dbd");
        m.insert("prestigelevelinfo", "PrestigeLevelInfo.dbd");
        m.insert("profession", "Profession.dbd");
        m.insert("professioneffect", "ProfessionEffect.dbd");
        m.insert("professioneffecttype", "ProfessionEffectType.dbd");
        m.insert("professionexpansion", "ProfessionExpansion.dbd");
        m.insert("professionrating", "ProfessionRating.dbd");
        m.insert("professiontrait", "ProfessionTrait.dbd");
        m.insert("professiontraitxeffect", "ProfessionTraitXEffect.dbd");
        m.insert("professiontraitxlabel", "ProfessionTraitXLabel.dbd");
        m.insert("professionxrating", "ProfessionXRating.dbd");
        m.insert("proftraitpathnode", "ProfTraitPathNode.dbd");
        m.insert("proftraitperknode", "ProfTraitPerkNode.dbd");
        m.insert("proftraittree", "ProfTraitTree.dbd");
        m.insert("proftraittreehighlight", "ProfTraitTreeHighlight.dbd");
        m.insert("pvpbrackettypes", "PVPBracketTypes.dbd");
        m.insert("pvpbrawl", "PvpBrawl.dbd");
        m.insert("pvpdifficulty", "PVPDifficulty.dbd");
        m.insert("pvpitem", "PVPItem.dbd");
        m.insert("pvprating", "PvpRating.dbd");
        m.insert("pvpreward", "PvpReward.dbd");
        m.insert("pvpscalingeffect", "PvpScalingEffect.dbd");
        m.insert("pvpscoreboardcellinfo", "PVPScoreboardCellInfo.dbd");
        m.insert("pvpscoreboardcolumnheader", "PVPScoreboardColumnHeader.dbd");
        m.insert("pvpscoreboardlayout", "PVPScoreboardLayout.dbd");
        m.insert("pvpseason", "PvpSeason.dbd");
        m.insert("pvpseasonrewardlevels", "PvpSeasonRewardLevels.dbd");
        m.insert("pvpstat", "PVPStat.dbd");
        m.insert("pvptalent", "PvpTalent.dbd");
        m.insert("pvptalentcategory", "PvpTalentCategory.dbd");
        m.insert("pvptalentslotunlock", "PvpTalentSlotUnlock.dbd");
        m.insert("pvptalentunlock", "PvpTalentUnlock.dbd");
        m.insert("pvptier", "PvpTier.dbd");
        m.insert("questdrivenscenario", "QuestDrivenScenario.dbd");
        m.insert("questfactionreward", "QuestFactionReward.dbd");
        m.insert("questfeedbackeffect", "QuestFeedbackEffect.dbd");
        m.insert("questhub", "QuestHub.dbd");
        m.insert("questinfo", "QuestInfo.dbd");
        m.insert("questline", "QuestLine.dbd");
        m.insert("questlinexquest", "QuestLineXQuest.dbd");
        m.insert("questmoneyreward", "QuestMoneyReward.dbd");
        m.insert("questobjective", "QuestObjective.dbd");
        m.insert("questpackageitem", "QuestPackageItem.dbd");
        m.insert("questpoiblob", "QuestPOIBlob.dbd");
        m.insert("questpoipoint", "QuestPOIPoint.dbd");
        m.insert("questsort", "QuestSort.dbd");
        m.insert("questv2", "QuestV2.dbd");
        m.insert("questv2clitask", "QuestV2CliTask.dbd");
        m.insert("questxgroupactivity", "QuestXGroupActivity.dbd");
        m.insert("questxp", "QuestXP.dbd");
        m.insert("questxuiquestdetailstheme", "QuestXUIQuestDetailsTheme.dbd");
        m.insert("questxuiwidgetset", "QuestXUiWidgetSet.dbd");
        m.insert("racialmounts", "RacialMounts.dbd");
        m.insert("rafactivity", "RafActivity.dbd");
        m.insert(
            "recipeprogressiongroupentry",
            "RecipeProgressionGroupEntry.dbd",
        );
        m.insert("relicslottierrequirement", "RelicSlotTierRequirement.dbd");
        m.insert("relictalent", "RelicTalent.dbd");
        m.insert("renownrewards", "RenownRewards.dbd");
        m.insert("renownrewardsplunderstorm", "RenownRewardsPlunderstorm.dbd");
        m.insert("researchbranch", "ResearchBranch.dbd");
        m.insert("researchfield", "ResearchField.dbd");
        m.insert("researchproject", "ResearchProject.dbd");
        m.insert("researchsite", "ResearchSite.dbd");
        m.insert("resistances", "Resistances.dbd");
        m.insert("rewardpack", "RewardPack.dbd");
        m.insert("rewardpackxcurrencytype", "RewardPackXCurrencyType.dbd");
        m.insert("rewardpackxitem", "RewardPackXItem.dbd");
        m.insert("ribbonquality", "RibbonQuality.dbd");
        m.insert("rolodextype", "RolodexType.dbd");
        m.insert("ropeeffect", "RopeEffect.dbd");
        m.insert("rtpcdata", "RTPCData.dbd");
        m.insert("rulesetitemupgrade", "RulesetItemUpgrade.dbd");
        m.insert("rulesetraidlootupgrade", "RulesetRaidLootUpgrade.dbd");
        m.insert("rulesetraidoverride", "RulesetRaidOverride.dbd");
        m.insert("runeforgelegendaryability", "RuneforgeLegendaryAbility.dbd");
        m.insert("sandboxscaling", "SandboxScaling.dbd");
        m.insert("scalingstatdistribution", "ScalingStatDistribution.dbd");
        m.insert("scalingstatvalues", "ScalingStatValues.dbd");
        m.insert("scenario", "Scenario.dbd");
        m.insert("scenarioevententry", "ScenarioEventEntry.dbd");
        m.insert("scenariostep", "ScenarioStep.dbd");
        m.insert("scenescript", "SceneScript.dbd");
        m.insert("scenescriptglobaltext", "SceneScriptGlobalText.dbd");
        m.insert("scenescriptpackage", "SceneScriptPackage.dbd");
        m.insert("scenescriptpackagemember", "SceneScriptPackageMember.dbd");
        m.insert("scheduledinterval", "ScheduledInterval.dbd");
        m.insert("scheduledworldstate", "ScheduledWorldState.dbd");
        m.insert("scheduledworldstategroup", "ScheduledWorldStateGroup.dbd");
        m.insert(
            "scheduledworldstatexuniqcat",
            "ScheduledWorldStateXUniqCat.dbd",
        );
        m.insert("screeneffect", "ScreenEffect.dbd");
        m.insert("screeneffecttype", "ScreenEffectType.dbd");
        m.insert("screenlocation", "ScreenLocation.dbd");
        m.insert("sdreplacementmodel", "SDReplacementModel.dbd");
        m.insert("seamlesssite", "SeamlessSite.dbd");
        m.insert("servermessages", "ServerMessages.dbd");
        m.insert("shadowyeffect", "ShadowyEffect.dbd");
        m.insert("sharedstring", "SharedString.dbd");
        m.insert("sheathesoundlookups", "SheatheSoundLookups.dbd");
        m.insert("siegeableproperties", "SiegeableProperties.dbd");
        m.insert("skillcostsdata", "SkillCostsData.dbd");
        m.insert("skilllineability", "SkillLineAbility.dbd");
        m.insert(
            "skilllineabilitysortedspell",
            "SkillLineAbilitySortedSpell.dbd",
        );
        m.insert("skilllinecategory", "SkillLineCategory.dbd");
        m.insert("skilllinextraittree", "SkillLineXTraitTree.dbd");
        m.insert("skillraceclassinfo", "SkillRaceClassInfo.dbd");
        m.insert("skilltiers", "SkillTiers.dbd");
        m.insert("skyscenexplayercondition", "SkySceneXPlayerCondition.dbd");
        m.insert("soulbind", "Soulbind.dbd");
        m.insert("soulbindconduit", "SoulbindConduit.dbd");
        m.insert(
            "soulbindconduitenhancedsocket",
            "SoulbindConduitEnhancedSocket.dbd",
        );
        m.insert("soulbindconduititem", "SoulbindConduitItem.dbd");
        m.insert("soulbindconduitrank", "SoulbindConduitRank.dbd");
        m.insert(
            "soulbindconduitrankproperties",
            "SoulbindConduitRankProperties.dbd",
        );
        m.insert("soulbinduidisplayinfo", "SoulbindUIDisplayInfo.dbd");
        m.insert("soundambience", "SoundAmbience.dbd");
        m.insert("soundbus", "SoundBus.dbd");
        m.insert("soundbusname", "SoundBusName.dbd");
        m.insert("soundbusoverride", "SoundBusOverride.dbd");
        m.insert("soundcharactermacrolines", "SoundCharacterMacroLines.dbd");
        m.insert("soundemitterpillpoints", "SoundEmitterPillPoints.dbd");
        m.insert("soundemitters", "SoundEmitters.dbd");
        m.insert("soundentries", "SoundEntries.dbd");
        m.insert("soundentriesadvanced", "SoundEntriesAdvanced.dbd");
        m.insert("soundentriesfallbacks", "SoundEntriesFallbacks.dbd");
        m.insert("soundenvelope", "SoundEnvelope.dbd");
        m.insert("soundfilter", "SoundFilter.dbd");
        m.insert("soundfilterelem", "SoundFilterElem.dbd");
        m.insert("soundkit", "SoundKit.dbd");
        m.insert("soundkitadvanced", "SoundKitAdvanced.dbd");
        m.insert("soundkitchild", "SoundKitChild.dbd");
        m.insert("soundkitentry", "SoundKitEntry.dbd");
        m.insert("soundkitname", "SoundKitName.dbd");
        m.insert("soundmixgroup", "SoundMixGroup.dbd");
        m.insert("soundoverride", "SoundOverride.dbd");
        m.insert("soundparameter", "SoundParameter.dbd");
        m.insert("soundproviderpreferences", "SoundProviderPreferences.dbd");
        m.insert("soundsamplepreferences", "SoundSamplePreferences.dbd");
        m.insert("soundwaterfallemitter", "SoundWaterfallEmitter.dbd");
        m.insert("soundwatertype", "SoundWaterType.dbd");
        m.insert("sourceinfo", "SourceInfo.dbd");
        m.insert("spammessages", "SpamMessages.dbd");
        m.insert("specializationspells", "SpecializationSpells.dbd");
        m.insert(
            "specializationspellsdisplay",
            "SpecializationSpellsDisplay.dbd",
        );
        m.insert("specsetmember", "SpecSetMember.dbd");
        m.insert("spell", "Spell.dbd");
        m.insert("spellactionbarpref", "SpellActionBarPref.dbd");
        m.insert("spellauranames", "SpellAuraNames.dbd");
        m.insert("spellauraoptions", "SpellAuraOptions.dbd");
        m.insert("spellaurarestrictions", "SpellAuraRestrictions.dbd");
        m.insert(
            "spellaurarestrictionsdifficulty",
            "SpellAuraRestrictionsDifficulty.dbd",
        );
        m.insert("spellauravisibility", "SpellAuraVisibility.dbd");
        m.insert("spellauravisxchrspec", "SpellAuraVisXChrSpec.dbd");
        m.insert("spellauravisxtalenttab", "SpellAuraVisXTalentTab.dbd");
        m.insert("spellcastingrequirements", "SpellCastingRequirements.dbd");
        m.insert("spellcasttimes", "SpellCastTimes.dbd");
        m.insert("spellcategories", "SpellCategories.dbd");
        m.insert("spellcategory", "SpellCategory.dbd");
        m.insert("spellchaineffects", "SpellChainEffects.dbd");
        m.insert("spellclassoptions", "SpellClassOptions.dbd");
        m.insert(
            "spellclutterareaeffectcounts",
            "SpellClutterAreaEffectCounts.dbd",
        );
        m.insert("spellclutterframerates", "SpellClutterFrameRates.dbd");
        m.insert("spellclutterkitdistances", "SpellClutterKitDistances.dbd");
        m.insert("spellcluttermissiledist", "SpellClutterMissileDist.dbd");
        m.insert(
            "spellclutterweapontraildist",
            "SpellClutterWeaponTrailDist.dbd",
        );
        m.insert("spellcooldowns", "SpellCooldowns.dbd");
        m.insert("spellcraftui", "SpellCraftUI.dbd");
        m.insert("spelldescriptionvariables", "SpellDescriptionVariables.dbd");
        m.insert("spelldifficulty", "SpellDifficulty.dbd");
        m.insert("spelldispeltype", "SpellDispelType.dbd");
        m.insert("spellduration", "SpellDuration.dbd");
        m.insert("spelleffect", "SpellEffect.dbd");
        m.insert(
            "spelleffectautodescription",
            "SpellEffectAutoDescription.dbd",
        );
        m.insert("spelleffectcamerashakes", "SpellEffectCameraShakes.dbd");
        m.insert("spelleffectemission", "SpellEffectEmission.dbd");
        m.insert("spelleffectgroupsize", "SpellEffectGroupSize.dbd");
        m.insert("spelleffectnames", "SpellEffectNames.dbd");
        m.insert("spellempower", "SpellEmpower.dbd");
        m.insert("spellempowerstage", "SpellEmpowerStage.dbd");
        m.insert("spellequippeditems", "SpellEquippedItems.dbd");
        m.insert("spellflyout", "SpellFlyout.dbd");
        m.insert("spellflyoutitem", "SpellFlyoutItem.dbd");
        m.insert("spellfocusobject", "SpellFocusObject.dbd");
        m.insert("spellicon", "SpellIcon.dbd");
        m.insert("spellinterrupts", "SpellInterrupts.dbd");
        m.insert("spellitemenchantment", "SpellItemEnchantment.dbd");
        m.insert(
            "spellitemenchantmentcondition",
            "SpellItemEnchantmentCondition.dbd",
        );
        m.insert("spellkeyboundoverride", "SpellKeyboundOverride.dbd");
        m.insert("spelllabel", "SpellLabel.dbd");
        m.insert("spelllearnspell", "SpellLearnSpell.dbd");
        m.insert("spelllevels", "SpellLevels.dbd");
        m.insert("spellmastery", "SpellMastery.dbd");
        m.insert("spellmechanic", "SpellMechanic.dbd");
        m.insert("spellmisc", "SpellMisc.dbd");
        m.insert("spellmiscdifficulty", "SpellMiscDifficulty.dbd");
        m.insert("spellmissile", "SpellMissile.dbd");
        m.insert("spellmissilemotion", "SpellMissileMotion.dbd");
        m.insert("spellname", "SpellName.dbd");
        m.insert("spelloverridename", "SpellOverrideName.dbd");
        m.insert("spellpower", "SpellPower.dbd");
        m.insert("spellpowerdifficulty", "SpellPowerDifficulty.dbd");
        m.insert("spellproceduraleffect", "SpellProceduralEffect.dbd");
        m.insert("spellprocsperminute", "SpellProcsPerMinute.dbd");
        m.insert("spellprocsperminutemod", "SpellProcsPerMinuteMod.dbd");
        m.insert("spellradius", "SpellRadius.dbd");
        m.insert("spellrange", "SpellRange.dbd");
        m.insert("spellreagents", "SpellReagents.dbd");
        m.insert("spellreagentscurrency", "SpellReagentsCurrency.dbd");
        m.insert("spellreplacement", "SpellReplacement.dbd");
        m.insert("spellscaling", "SpellScaling.dbd");
        m.insert("spellscript", "SpellScript.dbd");
        m.insert("spellscripttext", "SpellScriptText.dbd");
        m.insert("spellshapeshift", "SpellShapeshift.dbd");
        m.insert("spellshapeshiftform", "SpellShapeshiftForm.dbd");
        m.insert("spellspecialuniteffect", "SpellSpecialUnitEffect.dbd");
        m.insert("spelltargetrestrictions", "SpellTargetRestrictions.dbd");
        m.insert("spelltooltip", "SpellTooltip.dbd");
        m.insert("spelltotems", "SpellTotems.dbd");
        m.insert("spellvisual", "SpellVisual.dbd");
        m.insert("spellvisualanim", "SpellVisualAnim.dbd");
        m.insert("spellvisualanimname", "SpellVisualAnimName.dbd");
        m.insert("spellvisualcoloreffect", "SpellVisualColorEffect.dbd");
        m.insert("spellvisualeffectname", "SpellVisualEffectName.dbd");
        m.insert("spellvisualevent", "SpellVisualEvent.dbd");
        m.insert("playerdataelementaccount", "PlayerDataElementAccount.dbd");
        m.insert("professionproppoints", "ProfessionPropPoints.dbd");
        m.insert("pvpscalingeffecttype", "PvpScalingEffectType.dbd");
        m.insert("questlabel", "QuestLabel.dbd");
        m.insert("randproppoints", "RandPropPoints.dbd");
        m.insert("rtpc", "RTPC.dbd");
        m.insert("scenescripttext", "SceneScriptText.dbd");
        m.insert("skillline", "SkillLine.dbd");
        m.insert("soundambienceflavor", "SoundAmbienceFlavor.dbd");
        m.insert("soundkitfallback", "SoundKitFallback.dbd");
        m.insert("spellactivationoverlay", "SpellActivationOverlay.dbd");
        m.insert(
            "spellclutterimpactmodelcounts",
            "SpellClutterImpactModelCounts.dbd",
        );
        m.insert("spelleffectscaling", "SpellEffectScaling.dbd");
        m.insert("spellmemorizecost", "SpellMemorizeCost.dbd");
        m.insert("spellrunecost", "SpellRuneCost.dbd");
        m.insert("spellvisualkit", "SpellVisualKit.dbd");
        m.insert("stationery", "Stationery.dbd");
        m.insert("terraintypesounds", "TerrainTypeSounds.dbd");
        m.insert("traitedge", "TraitEdge.dbd");
        m.insert("traittreeloadoutentry", "TraitTreeLoadoutEntry.dbd");
        m.insert("trophytype", "TrophyType.dbd");
        m.insert("uieventtoast", "UIEventToast.dbd");
        m.insert("uimappoi", "UiMapPOI.dbd");
        m.insert("uitextureatlasmember", "UiTextureAtlasMember.dbd");
        m.insert("unittestsparse", "UnitTestSparse.dbd");
        m.insert("warbandscene", "WarbandScene.dbd");
        m.insert("wbpermissions", "WbPermissions.dbd");
        m.insert("worldlayermapset", "WorldLayerMapSet.dbd");
        m.insert(
            "contenttuningxexpectedstatmod",
            "ContentTuningXExpectedStatMod.dbd",
        );
        m.insert("itemarmorshield", "ItemArmorShield.dbd");
        m.insert("perksactivityxtag", "PerksActivityXTag.dbd");
        m.insert("spellvisualkitareamodel", "SpellVisualKitAreaModel.dbd");
        m.insert("spellvisualkiteffect", "SpellVisualKitEffect.dbd");
        m.insert("spellvisualkitmodelattach", "SpellVisualKitModelAttach.dbd");
        m.insert("spellvisualkitpicker", "SpellVisualKitPicker.dbd");
        m.insert("spellvisualkitpickerentry", "SpellVisualKitPickerEntry.dbd");
        m.insert("spellvisualmissile", "SpellVisualMissile.dbd");
        m.insert(
            "spellvisualprecasttransitions",
            "SpellVisualPrecastTransitions.dbd",
        );
        m.insert("spellvisualscreeneffect", "SpellVisualScreenEffect.dbd");
        m.insert(
            "spellxdescriptionvariables",
            "SpellXDescriptionVariables.dbd",
        );
        m.insert("spellxspellvisual", "SpellXSpellVisual.dbd");
        m.insert("spotlightconditionmap", "SpotLightConditionMap.dbd");
        m.insert("ssaosettings", "SSAOSettings.dbd");
        m.insert("stableslotprices", "StableSlotPrices.dbd");
        m.insert("startupfiles", "StartupFiles.dbd");
        m.insert("startup_strings", "Startup_Strings.dbd");
        m.insert("stringlookups", "StringLookups.dbd");
        m.insert("summonproperties", "SummonProperties.dbd");
        m.insert("tabardbackgroundtextures", "TabardBackgroundTextures.dbd");
        m.insert("tabardemblemtextures", "TabardEmblemTextures.dbd");
        m.insert("tactkey", "TactKey.dbd");
        m.insert("tactkeylookup", "TactKeyLookup.dbd");
        m.insert("talent", "Talent.dbd");
        m.insert("talenttab", "TalentTab.dbd");
        m.insert("talenttreeprimaryspells", "TalentTreePrimarySpells.dbd");
        m.insert("taxinodes", "TaxiNodes.dbd");
        m.insert("taxipath", "TaxiPath.dbd");
        m.insert("taxipathnode", "TaxiPathNode.dbd");
        m.insert("teamcontributionpoints", "TeamContributionPoints.dbd");
        m.insert("terraincolorgradingramp", "TerrainColorGradingRamp.dbd");
        m.insert("terrainmaterial", "TerrainMaterial.dbd");
        m.insert("terraintype", "TerrainType.dbd");
        m.insert("textureblendset", "TextureBlendSet.dbd");
        m.insert("texturefiledata", "TextureFileData.dbd");
        m.insert("tiertransition", "TierTransition.dbd");
        m.insert("timeeventdata", "TimeEventData.dbd");
        m.insert("totemcategory", "TotemCategory.dbd");
        m.insert("toy", "Toy.dbd");
        m.insert("tradeskillcategory", "TradeSkillCategory.dbd");
        m.insert("tradeskillitem", "TradeSkillItem.dbd");
        m.insert("traitcond", "TraitCond.dbd");
        m.insert("traitcondaccountelement", "TraitCondAccountElement.dbd");
        m.insert("traitcost", "TraitCost.dbd");
        m.insert("traitcostdefinition", "TraitCostDefinition.dbd");
        m.insert("traitcurrency", "TraitCurrency.dbd");
        m.insert("traitcurrencysource", "TraitCurrencySource.dbd");
        m.insert("traitdefinition", "TraitDefinition.dbd");
        m.insert(
            "traitdefinitioneffectpoints",
            "TraitDefinitionEffectPoints.dbd",
        );
        m.insert("traitnode", "TraitNode.dbd");
        m.insert("traitnodeentry", "TraitNodeEntry.dbd");
        m.insert("traitnodeentryxtraitcond", "TraitNodeEntryXTraitCond.dbd");
        m.insert("traitnodeentryxtraitcost", "TraitNodeEntryXTraitCost.dbd");
        m.insert("traitnodegroup", "TraitNodeGroup.dbd");
        m.insert("traitnodegroupxtraitcond", "TraitNodeGroupXTraitCond.dbd");
        m.insert("traitnodegroupxtraitcost", "TraitNodeGroupXTraitCost.dbd");
        m.insert("traitnodegroupxtraitnode", "TraitNodeGroupXTraitNode.dbd");
        m.insert("traitnodextraitcond", "TraitNodeXTraitCond.dbd");
        m.insert("traitnodextraitcost", "TraitNodeXTraitCost.dbd");
        m.insert("traitnodextraitnodeentry", "TraitNodeXTraitNodeEntry.dbd");
        m.insert("traitsubtree", "TraitSubTree.dbd");
        m.insert("traitsystem", "TraitSystem.dbd");
        m.insert("traittree", "TraitTree.dbd");
        m.insert("traittreeloadout", "TraitTreeLoadout.dbd");
        m.insert("traittreextraitcost", "TraitTreeXTraitCost.dbd");
        m.insert("traittreextraitcurrency", "TraitTreeXTraitCurrency.dbd");
        m.insert("transformmatrix", "TransformMatrix.dbd");
        m.insert("transmogdefaultlevel", "TransmogDefaultLevel.dbd");
        m.insert("transmogholiday", "TransmogHoliday.dbd");
        m.insert("transmogillusion", "TransmogIllusion.dbd");
        m.insert("transmogset", "TransmogSet.dbd");
        m.insert("transmogsetgroup", "TransmogSetGroup.dbd");
        m.insert("transmogsetitem", "TransmogSetItem.dbd");
        m.insert("transportanimation", "TransportAnimation.dbd");
        m.insert("transportphysics", "TransportPhysics.dbd");
        m.insert("transportrotation", "TransportRotation.dbd");
        m.insert("treasure", "Treasure.dbd");
        m.insert("trophy", "Trophy.dbd");
        m.insert("trophyinstance", "TrophyInstance.dbd");
        m.insert("uiarrowcallout", "UIArrowCallout.dbd");
        m.insert("uibutton", "UIButton.dbd");
        m.insert("uicamera", "UiCamera.dbd");
        m.insert("uicameratype", "UiCameraType.dbd");
        m.insert(
            "uicamfbacktalkingheadchrrace",
            "UiCamFbackTalkingHeadChrRace.dbd",
        );
        m.insert("uicamfbacktransmogchrrace", "UiCamFbackTransmogChrRace.dbd");
        m.insert("uicamfbacktransmogweapon", "UiCamFbackTransmogWeapon.dbd");
        m.insert("uicanvas", "UiCanvas.dbd");
        m.insert(
            "uichromietimeexpansioninfo",
            "UIChromieTimeExpansionInfo.dbd",
        );
        m.insert("uicinematicintroinfo", "UICinematicIntroInfo.dbd");
        m.insert("uicovenantability", "UICovenantAbility.dbd");
        m.insert("uicovenantdisplayinfo", "UiCovenantDisplayInfo.dbd");
        m.insert("uicovenantpreview", "UICovenantPreview.dbd");
        m.insert("uideadlydebuff", "UIDeadlyDebuff.dbd");
        m.insert("uidungeonscorerarity", "UIDungeonScoreRarity.dbd");
        m.insert("uiexpansiondisplayinfo", "UIExpansionDisplayInfo.dbd");
        m.insert(
            "uiexpansiondisplayinfoicon",
            "UIExpansionDisplayInfoIcon.dbd",
        );
        m.insert("uigenericwidgetdisplay", "UIGenericWidgetDisplay.dbd");
        m.insert("uiiteminteraction", "UiItemInteraction.dbd");
        m.insert("uimap", "UiMap.dbd");
        m.insert("uimapart", "UiMapArt.dbd");
        m.insert("uimapartstylelayer", "UiMapArtStyleLayer.dbd");
        m.insert("uimaparttile", "UiMapArtTile.dbd");
        m.insert("uimapassignment", "UiMapAssignment.dbd");
        m.insert("uimapfogofwar", "UiMapFogOfWar.dbd");
        m.insert(
            "uimapfogofwarvisualization",
            "UiMapFogOfWarVisualization.dbd",
        );
        m.insert("uimapgroup", "UiMapGroup.dbd");
        m.insert("uimapgroupmember", "UiMapGroupMember.dbd");
        m.insert("uimaplink", "UiMapLink.dbd");
        m.insert("uimappininfo", "UIMapPinInfo.dbd");
        m.insert("uimapxmapart", "UiMapXMapArt.dbd");
        m.insert("uimodelscene", "UiModelScene.dbd");
        m.insert("uimodelsceneactor", "UiModelSceneActor.dbd");
        m.insert("uimodelsceneactordisplay", "UiModelSceneActorDisplay.dbd");
        m.insert("uimodelscenecamera", "UiModelSceneCamera.dbd");
        m.insert("uimodifiedinstance", "UIModifiedInstance.dbd");
        m.insert("uipartypose", "UiPartyPose.dbd");
        m.insert("uiquestdetailstheme", "UiQuestDetailsTheme.dbd");
        m.insert("uiscriptedanimationeffect", "UIScriptedAnimationEffect.dbd");
        m.insert("uisoundlookups", "UISoundLookups.dbd");
        m.insert("uisplashscreen", "UISplashScreen.dbd");
        m.insert("uitextureatlas", "UiTextureAtlas.dbd");
        m.insert("uitextureatlaselement", "UiTextureAtlasElement.dbd");
        m.insert(
            "uitextureatlaselementoverride",
            "UiTextureAtlasElementOverride.dbd",
        );
        m.insert(
            "uitextureatlaselementslicedata",
            "UiTextureAtlasElementSliceData.dbd",
        );
        m.insert("uitexturekit", "UiTextureKit.dbd");
        m.insert("uiweeklyreward", "UiWeeklyReward.dbd");
        m.insert("uiwidget", "UiWidget.dbd");
        m.insert("uiwidgetconstantsource", "UiWidgetConstantSource.dbd");
        m.insert("uiwidgetdatasource", "UiWidgetDataSource.dbd");
        m.insert("uiwidgetmap", "UiWidgetMap.dbd");
        m.insert("uiwidgetset", "UiWidgetSet.dbd");
        m.insert("uiwidgetstringsource", "UiWidgetStringSource.dbd");
        m.insert("uiwidgetvistypedatareq", "UiWidgetVisTypeDataReq.dbd");
        m.insert("uiwidgetvisualization", "UiWidgetVisualization.dbd");
        m.insert("uiwidgetxwidgetset", "UiWidgetXWidgetSet.dbd");
        m.insert("unitblood", "UnitBlood.dbd");
        m.insert("unitbloodlevels", "UnitBloodLevels.dbd");
        m.insert("unitcondition", "UnitCondition.dbd");
        m.insert("unitpowerbar", "UnitPowerBar.dbd");
        m.insert("unittest", "UnitTest.dbd");
        m.insert("vehicle", "Vehicle.dbd");
        m.insert("vehiclepoitype", "VehiclePOIType.dbd");
        m.insert("vehicleseat", "VehicleSeat.dbd");
        m.insert("vehicleuiindicator", "VehicleUIIndicator.dbd");
        m.insert("vehicleuiindseat", "VehicleUIIndSeat.dbd");
        m.insert("videohardware", "VideoHardware.dbd");
        m.insert("vignette", "Vignette.dbd");
        m.insert("vignetteuiwidgetset", "VignetteUiWidgetSet.dbd");
        m.insert("virtualattachment", "VirtualAttachment.dbd");
        m.insert(
            "virtualattachmentcustomization",
            "VirtualAttachmentCustomization.dbd",
        );
        m.insert("vocaluisounds", "VocalUISounds.dbd");
        m.insert("voiceoverpriority", "VoiceOverPriority.dbd");
        m.insert("volumefogcondition", "VolumeFogCondition.dbd");
        m.insert("vw_mobilespell", "VW_MobileSpell.dbd");
        m.insert(
            "warbandplacementdisplayinfo",
            "WarbandPlacementDisplayInfo.dbd",
        );
        m.insert("warbandsceneanimation", "WarbandSceneAnimation.dbd");
        m.insert("warbandsceneanimchrspec", "WarbandSceneAnimChrSpec.dbd");
        m.insert("warbandsceneplacement", "WarbandScenePlacement.dbd");
        m.insert(
            "warbandsceneplacementfilterreq",
            "WarbandScenePlacementFilterReq.dbd",
        );
        m.insert(
            "warbandsceneplacementoption",
            "WarbandScenePlacementOption.dbd",
        );
        m.insert(
            "warbandsceneplcmntanimoverride",
            "WarbandScenePlcmntAnimOverride.dbd",
        );
        m.insert("warbandscenesourceinfo", "WarbandSceneSourceInfo.dbd");
        m.insert("waterfalldata", "WaterfallData.dbd");
        m.insert("waypointedge", "WaypointEdge.dbd");
        m.insert("waypointmapvolume", "WaypointMapVolume.dbd");
        m.insert("waypointnode", "WaypointNode.dbd");
        m.insert("waypointsafelocs", "WaypointSafeLocs.dbd");
        m.insert("wbaccesscontrollist", "WbAccessControlList.dbd");
        m.insert("wbcertblacklist", "WbCertBlacklist.dbd");
        m.insert("wbcertwhitelist", "WbCertWhitelist.dbd");
        m.insert("weaponimpactsounds", "WeaponImpactSounds.dbd");
        m.insert("weaponswingsounds2", "WeaponSwingSounds2.dbd");
        m.insert("weapontrail", "WeaponTrail.dbd");
        m.insert("weapontrailmodeldef", "WeaponTrailModelDef.dbd");
        m.insert("weapontrailparam", "WeaponTrailParam.dbd");
        m.insert("weather", "Weather.dbd");
        m.insert("weatherxparticulate", "WeatherXParticulate.dbd");
        m.insert(
            "weeklyrewardchestactivitytier",
            "WeeklyRewardChestActivityTier.dbd",
        );
        m.insert(
            "weeklyrewardchestthreshold",
            "WeeklyRewardChestThreshold.dbd",
        );
        m.insert("windsettings", "WindSettings.dbd");
        m.insert("wmoareatable", "WMOAreaTable.dbd");
        m.insert("wmominimaptexture", "WMOMinimapTexture.dbd");
        m.insert("worldbosslockout", "WorldBossLockout.dbd");
        m.insert("worldchunksounds", "WorldChunkSounds.dbd");
        m.insert("worldeffect", "WorldEffect.dbd");
        m.insert("worldelapsedtimer", "WorldElapsedTimer.dbd");
        m.insert("worldmaparea", "WorldMapArea.dbd");
        m.insert("worldmapcontinent", "WorldMapContinent.dbd");
        m.insert("worldmapoverlay", "WorldMapOverlay.dbd");
        m.insert("worldmapoverlaytile", "WorldMapOverlayTile.dbd");
        m.insert("worldmaptransforms", "WorldMapTransforms.dbd");
        m.insert("worldsafelocs", "WorldSafeLocs.dbd");
        m.insert("worldshadow", "WorldShadow.dbd");
        m.insert("worldstate", "WorldState.dbd");
        m.insert("worldstateexpression", "WorldStateExpression.dbd");
        m.insert("worldstateui", "WorldStateUI.dbd");
        m.insert("worldstatezonesounds", "WorldStateZoneSounds.dbd");
        m.insert("world_pvp_area", "World_PVP_Area.dbd");
        m.insert("wowerror_strings", "WowError_Strings.dbd");
        m.insert("zoneintromusictable", "ZoneIntroMusicTable.dbd");
        m.insert("zonelight", "ZoneLight.dbd");
        m.insert("zonelightpoint", "ZoneLightPoint.dbd");
        m.insert("zonemusic", "ZoneMusic.dbd");
        m.insert("zonestory", "ZoneStory.dbd");
        m.insert("guildcolorborder", "GuildColorBorder.dbd");
        m.insert("guildcoloremblem", "GuildColorEmblem.dbd");
        m.insert("guildemblem", "GuildEmblem.dbd");
        m.insert("guildperkspells", "GuildPerkSpells.dbd");
        m.insert("guildshirtbackground", "GuildShirtBackground.dbd");
        m.insert("guildshirtborder", "GuildShirtBorder.dbd");
        m.insert("guildtabardbackground", "GuildTabardBackground.dbd");
        m.insert("guildtabardborder", "GuildTabardBorder.dbd");
        m.insert("guildtabardemblem", "GuildTabardEmblem.dbd");
        m.insert("heirloom", "Heirloom.dbd");
        m.insert("helmetanimscaling", "HelmetAnimScaling.dbd");
        m.insert("helmetgeosetdata", "HelmetGeosetData.dbd");
        m.insert("helmetgeosetvisdata", "HelmetGeosetVisData.dbd");
        m.insert("highlightcolor", "HighlightColor.dbd");

        m
    })
}

pub fn get_dbd_name(file_name: &str) -> Result<&'static str> {
    let mut file_name = file_name.to_lowercase();
    if let Some(idx) = file_name.find(".") {
        file_name.truncate(idx);
    };

    let res: &'static str = get_db_file_map()
        .get(&file_name.as_str())
        .ok_or_else(|| Error::SchemaValidation("".into()))?;

    Ok(res)
}
