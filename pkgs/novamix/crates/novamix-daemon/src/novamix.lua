-- Written by novamix-daemon; edits are overwritten.
--
-- Sends each new playback stream that matches novamix.rules to GAME or CHAT. It runs
-- when the stream appears, after WirePlumber restored its saved target and before it links
-- it, the same way restore-stream does, so the stream never plays elsewhere first.

local log = Log.open_topic ("s-novamix")
local rules = Conf.get_section_as_json ("novamix.rules", Json.Array {})

SimpleEventHook {
  name = "node/novamix-route",
  after = "node/restore-stream",
  interests = {
    EventInterest {
      Constraint { "event.type", "=", "node-added" },
      Constraint { "media.class", "=", "Stream/Output/Audio" },
    },
  },
  execute = function (event)
    local node = event:get_subject ()
    local props = node.properties
    -- The app asked for a sink itself
    if props ["target.object"] or props ["node.target"] then
      return
    end

    local target = nil
    JsonUtils.match_rules (rules, props, function (action, value)
      if action == "novamix-route" and not target then
        target = value:parse ()
      end
      return true
    end)
    if not target then
      return
    end

    local source = event:get_source ()
    local sink = source:call ("get-object-manager", "node"):lookup {
      Constraint { "node.name", "=", target, type = "pw" },
    }
    local metadata = source:call ("get-object-manager", "metadata"):lookup {
      Constraint { "metadata.name", "=", "default" },
    }
    -- No sink while the daemon is not running: the stream goes where it would anyway
    if not sink or not metadata then
      return
    end
    log:info (node, tostring (props ["node.name"]) .. " -> " .. target)
    metadata:set (node ["bound-id"], "target.object", "Spa:Id",
        sink.properties ["object.serial"])
  end
}:register ()
