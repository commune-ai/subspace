#!/bin/bash
# This script is meant to be run on Unix/Linux based systems
docker compose -d up


while :; do
    case $1 in

        --build)
        FORCEBUILD="true"
        
        ;;

        --down)
            printf $COLOR_R'Doing a deep clean ...\n\n'$COLOR_RESET
            # eval docker network rm ${PROJECT_NAME}_default || true;
            eval docker-compose "$COMPOSE_FILES" down;
            break;
        ;;

        
        --restart)

           
            break
            ;;

        --) # End of all options.
            shift
            break
            ;;
        -?*)
            printf $COLOR_R'WARN: Unknown option (ignored): %s\n'$COLOR_RESET "$1" >&2
            break
            ;;
        *)
            break
    esac
    shift
done



