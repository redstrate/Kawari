-- Your SQL goes here
CREATE TABLE `user`(
	`id` BIGINT NOT NULL PRIMARY KEY,
	`username` TEXT NOT NULL,
	`password` TEXT NOT NULL
);

CREATE TABLE `session`(
	`user_id` BIGINT NOT NULL,
	`time` TEXT NOT NULL,
	`service` TEXT NOT NULL,
	`sid` TEXT NOT NULL,
	PRIMARY KEY(`user_id`, `service`),
	FOREIGN KEY (`user_id`) REFERENCES `user`(`id`)
);

CREATE TABLE `service_account`(
	`id` BIGINT NOT NULL PRIMARY KEY,
	`user_id` BIGINT NOT NULL,
	`max_ex` INTEGER NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `user`(`id`)
);

