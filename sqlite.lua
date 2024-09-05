local varint = function(buffer, start)
	local vr = 0

	for i = 0, 8, 1 do
		local byte = buffer.read_be(start + i)

		if (byte & 0x80) == 0 or i == 8 then
			return vr << 7 | byte, i + 1
		end

		vr = vr << 7 | (byte & 0x7f)
	end
end

local content_size = function(value)
	if value <= 4 then
		return value
	elseif value == 5 then
		return 6
	elseif value == 6 or value == 7 then
		return 8
	elseif value <= 11 then
		return 0
	elseif value % 2 == 0 then
		return (value - 12) // 2
	else
		return (value - 13) // 2
	end
end

return function(buffer)
	local page_size = buffer.read_be(16, 18)
	local pages = buffer.read_be(28, 32)

	for i = 0, pages - 1, 1 do
		local page = page_size * i

		buffer.color({ page, page + page_size, text = "page " .. i })

		local btree_page = page

		if i == 0 then
			buffer.color({ page, page + 100, bg = "#303030", text = "header" })
			btree_page = btree_page + 100
		end

		local page_type = buffer.read_be(page)
		local btree_header = btree_page

		if page_type == 0x05 then
			btree_header = btree_header + 12
		else
			btree_header = btree_header + 8
		end

		buffer.color({ btree_page, btree_header, bg = "#505050", text = "page header" })

		local cell_number = buffer.read_be(btree_page + 3, btree_page + 5)
		buffer.color({ btree_page, fg = "#d0d000", text = "btree page type" })
		buffer.color({ btree_page + 3, btree_page + 5, fg = "#ff0000", text = "#cells" })

		for i = 0, cell_number - 1, 1 do
			buffer.color({
				a = btree_header + i * 2,
				b = btree_header + (i + 1) * 2,
				bg = "#008000",
				text = "cell #" .. i .. " pointer",
			})

			local offset = buffer.read_be(btree_header + i * 2, btree_header + (i + 1) * 2)
			local cell = page + offset

			offset = 0

			local bytes, len = varint(buffer, cell + offset)
			buffer.color({ cell + offset, cell + offset + len, fg = "#ff00ff", text = "payload bytes" })
			offset = offset + len

			local rowid, len = varint(buffer, cell + offset)
			buffer.color({ cell + offset, cell + offset + len, fg = "#ff0000", text = "rowid" })
			offset = offset + len

			local schema_len, len = varint(buffer, cell + offset)
			buffer.color({ cell + offset, cell + offset + schema_len, bg = "#d0d000", fg = "#000000", text = "payload" })
			local current_len = len

			local vrs = {}
			local idx = 1

			i = 1
			while current_len < schema_len do
				local vr, len = varint(buffer, cell + offset + current_len)

				vrs[idx] = vr
				idx = idx + 1

				buffer.color({
					cell + offset + current_len,
					cell + offset + current_len + len,
					bg = "#0000ff",
					fg = "#ffffff",
					text = "serial #" .. i,
				})

				current_len = current_len + len
				i = i + 1
			end

			offset = offset + schema_len

			for i, value in pairs(vrs) do
				local size = content_size(value)
				buffer.color({ cell + offset, cell + offset + size, bg = "#303030", text = "record #" .. i })
				offset = offset + size
			end
		end
	end
end
