-- Your SQL goes here
CREATE TABLE `classjob`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`classjob_id` INTEGER NOT NULL,
	`classjob_levels` TEXT NOT NULL,
	`classjob_exp` TEXT NOT NULL,
	`first_classjob` INTEGER NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `customize`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`chara_make` TEXT NOT NULL,
	`city_state` INTEGER NOT NULL,
	`remake_mode` INTEGER NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `inventory`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`contents` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `companion`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`unlocked_equip` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `volatile`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`pos_x` DOUBLE NOT NULL,
	`pos_y` DOUBLE NOT NULL,
	`pos_z` DOUBLE NOT NULL,
	`rotation` DOUBLE NOT NULL,
	`zone_id` INTEGER NOT NULL,
	`display_flags` INTEGER NOT NULL,
	`title` INTEGER NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `character`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`service_account_id` BIGINT NOT NULL,
	`actor_id` BIGINT NOT NULL,
	`gm_rank` INTEGER NOT NULL,
	`name` TEXT NOT NULL
);

CREATE TABLE `quest`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`completed` TEXT NOT NULL,
	`active` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `unlock`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`unlocks` TEXT NOT NULL,
	`seen_active_help` TEXT NOT NULL,
	`minions` TEXT NOT NULL,
	`mounts` TEXT NOT NULL,
	`orchestrion_rolls` TEXT NOT NULL,
	`cutscene_seen` TEXT NOT NULL,
	`ornaments` TEXT NOT NULL,
	`caught_fish` TEXT NOT NULL,
	`caught_spearfish` TEXT NOT NULL,
	`adventures` TEXT NOT NULL,
	`triple_triad_cards` TEXT NOT NULL,
	`glasses_styles` TEXT NOT NULL,
	`chocobo_taxi_stands` TEXT NOT NULL,
	`titles` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `aether_current`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`comp_flg_set` TEXT NOT NULL,
	`unlocked` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `aetheryte`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`unlocked` TEXT NOT NULL,
	`homepoint` INTEGER NOT NULL,
	`favorite_aetherytes` TEXT NOT NULL,
	`free_aetheryte` INTEGER NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

CREATE TABLE `content`(
	`content_id` BIGINT NOT NULL PRIMARY KEY,
	`unlocked_raids` TEXT NOT NULL,
	`unlocked_dungeons` TEXT NOT NULL,
	`unlocked_guildhests` TEXT NOT NULL,
	`unlocked_trials` TEXT NOT NULL,
	`unlocked_crystalline_conflicts` TEXT NOT NULL,
	`unlocked_frontlines` TEXT NOT NULL,
	`cleared_raids` TEXT NOT NULL,
	`cleared_dungeons` TEXT NOT NULL,
	`cleared_guildhests` TEXT NOT NULL,
	`cleared_trials` TEXT NOT NULL,
	`cleared_crystalline_conflicts` TEXT NOT NULL,
	`cleared_frontlines` TEXT NOT NULL,
	FOREIGN KEY (`content_id`) REFERENCES `character`(`content_id`)
);

