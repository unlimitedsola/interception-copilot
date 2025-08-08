cmake_minimum_required(VERSION 3.15)

project(interception C)

# Language standard
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

# Sources
set(INTERCEPTION_PUBLIC_HEADERS
	${CMAKE_CURRENT_SOURCE_DIR}/interception.h
)

set(INTERCEPTION_COMMON_SOURCES
	${CMAKE_CURRENT_SOURCE_DIR}/interception.c
	${INTERCEPTION_PUBLIC_HEADERS}
)

# Resource file
set(INTERCEPTION_RC ${CMAKE_CURRENT_SOURCE_DIR}/interception.rc)
set(INTERCEPTION_RC_SOURCES)
if(EXISTS ${INTERCEPTION_RC})
	list(APPEND INTERCEPTION_RC_SOURCES ${INTERCEPTION_RC})
endif()

# Shared library (DLL)
add_library(interception SHARED
	${INTERCEPTION_COMMON_SOURCES}
	${INTERCEPTION_RC_SOURCES}
)

target_include_directories(interception PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

# Match original 'sources' definitions: export symbols when building the DLL
target_compile_definitions(interception PRIVATE INTERCEPTION_EXPORT)

# Link system libraries used by interception.c
if(WIN32)
	target_link_libraries(interception PRIVATE kernel32 advapi32)
endif()

# Static library
add_library(interception_static STATIC
	${INTERCEPTION_COMMON_SOURCES}
)

target_include_directories(interception_static PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

# Static build uses INTERCEPTION_STATIC so headers don't mark imports/exports
target_compile_definitions(interception_static PUBLIC INTERCEPTION_STATIC)

# Avoid name collision between DLL import lib and static lib on Windows
set_target_properties(interception_static PROPERTIES OUTPUT_NAME interception_static)

# Keep DLL named simply 'interception'
set_target_properties(interception PROPERTIES OUTPUT_NAME interception)

# Nice-to-have: organize output dirs if user builds in-source on Windows
if(WIN32)
	foreach(tgt IN ITEMS interception interception_static)
		set_target_properties(${tgt} PROPERTIES
			ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib
			LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
			RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
		)
	endforeach()
endif()


