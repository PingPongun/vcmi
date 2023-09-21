
if(CMAKE_BUILD_TYPE MATCHES Debug)
	set(VCMI_CARGO_PROFILE "dev")
elseif(CMAKE_BUILD_TYPE MATCHES MinSizeRel)
	set(VCMI_CARGO_PROFILE "MinSizeRel")
elseif(CMAKE_BUILD_TYPE MATCHES RelWithDebInfo)
	set(VCMI_CARGO_PROFILE "RelWithDebInfo")
else()
	set(VCMI_CARGO_PROFILE "release")
endif()

if(ANDROID)
	#TODO teoriticly it should be possible to make corrosion work also with android. 
	add_custom_target(
		vcmilauncherv2 ALL
		COMMAND cargo bandroid --profile=${VCMI_CARGO_PROFILE}
		WORKING_DIRECTORY  ${CMAKE_SOURCE_DIR}/launcher_v2
		COMMENT "Compiling launcher_v2 with cargo..."
	)
else()
	include(FetchContent)

	FetchContent_Declare(
		Corrosion
		GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
		GIT_TAG v0.4 # Optionally specify a commit hash, version tag or branch here
	)
	FetchContent_MakeAvailable(Corrosion)

	if(APPLE_IOS)
		# Import targets defined in a package or workspace manifest `Cargo.toml` file
		corrosion_import_crate(
			MANIFEST_PATH ${CMAKE_SOURCE_DIR}/launcher_v2/Cargo.toml
			PROFILE ${VCMI_CARGO_PROFILE}
			CRATE_TYPES "staticlib"
		)
	else()
		# Import targets defined in a package or workspace manifest `Cargo.toml` file
		corrosion_import_crate(
			MANIFEST_PATH ${CMAKE_SOURCE_DIR}/launcher_v2/Cargo.toml 
			PROFILE ${VCMI_CARGO_PROFILE}
			CRATE_TYPES "bin"
			FEATURES "desktop"
		)
	endif()
endif()


if(WIN32)
	set_target_properties(vcmilauncherv2
		PROPERTIES
			OUTPUT_NAME "VCMI_launcher"
			PROJECT_LABEL "VCMI_launcher"
	)
endif()

if(APPLE)
	# This makes Xcode project prettier by moving vcmilauncherv2_autogen directory into vcmiclient subfolder
	set_property(GLOBAL PROPERTY AUTOGEN_TARGETS_FOLDER vcmilauncherv2)
endif()

vcmi_set_output_dir(vcmilauncherv2 "")

if(APPLE_IOS)
	set(RESOURCES_DESTINATION ${DATA_DIR})

else()
	set(RESOURCES_DESTINATION ${DATA_DIR}/launcher)

	# Install icons and desktop file on Linux
	if(NOT WIN32 AND NOT APPLE)
		install(FILES "vcmilauncherv2.desktop" DESTINATION share/applications)
		install(FILES "eu.vcmi.VCMI.metainfo.xml" DESTINATION share/metainfo)
	endif()
endif()

