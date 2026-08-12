package google

import T "github.com/medulla-sh/cuet"

#CloudSqlPostgresVersion:
	14 |
	15 |
	16 |
	17 |
	18

#CloudSqlEditionMap: {
	enterprise:        "ENTERPRISE"
	"enterprise-plus": "ENTERPRISE_PLUS"
}

#CloudSqlAvailabilityMap: {
	zonal:    "ZONAL"
	regional: "REGIONAL"
}

#CloudSqlStorageTypeMap: {
	ssd: "PD_SSD"
	hdd: "PD_HDD"
}

#CloudSqlInstance: {
	in: {
		#import?: {
			instance?: string
			databases?: [string]: string
			users?: [string]:     string
		}

		name:     #RFC1035Name
		project?: string
		region:   #Region
		engine: {
			type:    "postgres"
			version: #CloudSqlPostgresVersion
		}

		tier: string & =~"^db-[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$"

		edition: or([for name, _ in #CloudSqlEditionMap {name}])
		edition: _ | *"enterprise"

		availability: or([for name, _ in #CloudSqlAvailabilityMap {name}])
		availability: _ | *"zonal"

		storage: {
			sizeGb: int & >=10
			sizeGb: _ | *10

			type: or([for name, _ in #CloudSqlStorageTypeMap {name}])
			type: _ | *"ssd"

			autoResize: bool
			autoResize: _ | *true
		}
		storage: _ | *{}

		backups: {
			enabled: bool
			enabled: _ | *true

			pointInTimeRecovery: bool
			pointInTimeRecovery: _ | *true
		}

		deletionProtection: bool
		deletionProtection: _ | *true

		privateNetwork: {
			id:              string
			allocatedRange?: string
		}

		labels: {[string]: string}

		databases: [string & !=""]: {}
		users: [string & !=""]: {
			password: string
		}
	}

	let instanceRef = "google_sql_database_instance.\(in.name)"
	let databaseResourceNames = {
		for name, _ in in.databases {
			(name): "\(in.name)-\(name)"
		}
	}
	let userResourceNames = {
		for name, _ in in.users {
			(name): "\(in.name)-\(name)"
		}
	}

	refs: {
		instance: instanceRef
		databases: {
			for name, resourceName in databaseResourceNames {
				(name): "google_sql_database.\(resourceName)"
			}
		}
		users: {
			for name, resourceName in userResourceNames {
				(name): "google_sql_user.\(resourceName)"
			}
		}
	}

	out: T.#TerraformInput & {
		resource: google_sql_database_instance: (in.name): {
			if in.#import.instance != _|_ {
				#import: in.#import.instance
			}

			name:             in.name
			region:           in.region
			database_version: "POSTGRES_\(in.engine.version)"

			if in.project != _|_ {
				project: in.project
			}

			deletion_protection: in.deletionProtection

			settings: {
				tier:              in.tier
				edition:           #CloudSqlEditionMap[in.edition]
				availability_type: #CloudSqlAvailabilityMap[in.availability]
				disk_size:         in.storage.sizeGb
				disk_type:         #CloudSqlStorageTypeMap[in.storage.type]
				disk_autoresize:   in.storage.autoResize

				deletion_protection_enabled: in.deletionProtection

				if len(in.labels) > 0 {
					user_labels: in.labels
				}

				backup_configuration: {
					enabled:                        in.backups.enabled
					point_in_time_recovery_enabled: in.backups.pointInTimeRecovery
				}

				ip_configuration: {
					ipv4_enabled:    false
					private_network: in.privateNetwork.id
					ssl_mode:        "ENCRYPTED_ONLY"

					if in.privateNetwork.allocatedRange != _|_ {
						allocated_ip_range: in.privateNetwork.allocatedRange
					}
				}
			}
		}

		for name, resourceName in databaseResourceNames {
			resource: google_sql_database: (resourceName): {
				if in.#import.databases[name] != _|_ {
					#import: in.#import.databases[name]
				}

				"name":   name
				instance: "${\(instanceRef).name}"

				if in.project != _|_ {
					project: in.project
				}
			}
		}

		for name, user in in.users {
			let resourceName = userResourceNames[name]
			resource: google_sql_user: (resourceName): {
				if in.#import.users[name] != _|_ {
					#import: in.#import.users[name]
				}

				"name":   name
				instance: "${\(instanceRef).name}"
				password: user.password

				if in.project != _|_ {
					project: in.project
				}
			}
		}
	}
}
