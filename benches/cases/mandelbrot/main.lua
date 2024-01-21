local size = 500

local sum = 0
local byte_acc = 0
local bit_num = 0

local y = 0

while y < size do
    local ci = (2.0 * y / size) - 1.0
    local x  = 0

    while x < size do
        local zrzr = 0.0
        local zi = 0.0
        local zizi = 0.0
        local cr = (2.0 * x / size) - 1.5

        local z = 0
        local not_done = true
        local escape = 0
        while not_done and z < 50 do
            local zr = zrzr - zizi + cr
            zi = 2.0 * zr * zi + ci

            zrzr = zr * zr
            zizi = zi * zi

            if zrzr + zizi > 4.0 then
                not_done = false
                escape = 1
            end
            z = z + 1
        end

        byte_acc = (byte_acc << 1) + escape
        bit_num = bit_num + 1

        if bit_num == 8 then
            sum = sum ~ byte_acc
            byte_acc = 0
            bit_num = 0
        elseif x == size - 1 then
            byte_acc = byte_acc << (8 - bit_num)
            sum = sum ~ byte_acc
            byte_acc = 0
            bit_num = 0
        end
        x = x + 1
    end
    y = y + 1
end

print(sum)
