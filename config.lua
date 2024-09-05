local equals = function(a, b)
	for k, v in pairs(a) do
		if b[k] ~= v then
			return false
		end
	end

	return true
end

return {
	page = 4096,
	endian = "big",

	highlight = function(buffer)
		local header = buffer.read(0, 6)

		--                  Sqlite
		if equals(header, { 0x53, 0x51, 0x4c, 0x69, 0x74, 0x65 }) then
			require("sqlite")(buffer)
		end
	end,
}
